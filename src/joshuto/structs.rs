extern crate ncurses;

use std;
use std::fs;
use std::ffi;

use joshuto;
use joshuto::unix;

pub struct JoshutoDirEntry {
    pub index           : usize,
    pub metadata        : fs::Metadata,
    pub dir_contents    : Option<Vec<fs::DirEntry>>,
}

impl JoshutoDirEntry {

    pub fn new(path : &str, show_hidden : bool)
        -> Result<JoshutoDirEntry, std::io::Error>
    {
        let dir_contents : Vec<fs::DirEntry>;
        if show_hidden {
            dir_contents = joshuto::list_dirent_hidden(path)?;
        } else {
            dir_contents = joshuto::list_dirent(path)?;
        }

        let file = fs::File::open(path)?;
        let metadata = file.metadata()?;

        Ok(JoshutoDirEntry {
            index: 0,
            metadata: metadata,
            dir_contents: Some(dir_contents),
        })
    }
}

pub struct JoshutoWindow {
    pub win     : ncurses::WINDOW,
    pub rows    : i32,
    pub cols    : i32,
    pub coords  : (i32, i32)
}

impl JoshutoWindow {
    pub fn new(rows : i32, cols : i32, coords : (i32, i32)) -> JoshutoWindow
    {
        let win = ncurses::newwin(rows, cols, coords.0, coords.1);

        ncurses::refresh();
        JoshutoWindow {
            win: win,
            rows: rows,
            cols: cols,
            coords: coords,
        }
    }

    pub fn redraw(&mut self, rows : i32, cols : i32, coords : (i32, i32))
    {
        ncurses::delwin(self.win);
        self.rows = rows;
        self.cols = cols;
        self.coords = coords;
        self.win = ncurses::newwin(self.rows, self.cols, self.coords.0,
                self.coords.1);
    }

    pub fn commit(&self)
    {
        ncurses::wrefresh(self.win);
    }

    fn print_file(&self, file : &fs::DirEntry)
    {
        use std::os::unix::fs::PermissionsExt;

        let mut mode : u32 = 0;

        if let Ok(metadata) = file.metadata() {
            mode = metadata.permissions().mode();
        }
        if mode != 0 {
            file_attr_apply(self.win, mode, file.path().extension(), ncurses::wattron);
        }

        match file.file_name().into_string() {
            Ok(file_name) => {
                ncurses::wprintw(self.win, " ");
                if file_name.len() + 1 >= self.cols as usize {
                    let mut shortened = String::with_capacity(
                            self.cols as usize - 4);
                    let mut iter = file_name.chars();
                    for _i in 0..self.cols - 5 {
                        if let Some(ch) = iter.next() {
                            shortened.push(ch);
                        }
                    }
                    ncurses::wprintw(self.win, &shortened);
                    ncurses::wprintw(self.win, "…");
                } else {
                    ncurses::wprintw(self.win, &file_name);
                }
            },
            Err(e) => {
                ncurses::wprintw(self.win, format!("{:?}", e).as_str());
            },
        };

        if mode != 0 {
            file_attr_apply(self.win, mode, file.path().extension(),
                    ncurses::wattroff);
        }
        ncurses::waddstr(self.win, "\n");
    }

    pub fn display_contents(&self, dir_contents: &Vec<fs::DirEntry>,
            index : usize) {
        let vec_len = dir_contents.len();
        if vec_len == 0 {
            joshuto::wprintmsg(self, "empty");
            return;
        }

        let offset : usize = 5;
        let start : usize;
        let end : usize;

        if self.rows as usize >= vec_len {
            start = 0;
            end = vec_len;
        } else if index <= offset {
            start = 0;
            end = self.rows as usize;
        } else if index - offset + self.rows as usize >= vec_len {
            start = vec_len - self.rows as usize;
            end = vec_len;
        } else {
            start = index - offset;
            end = start + self.rows as usize;
        }

        ncurses::wclear(self.win);
        ncurses::wmove(self.win, 0, 0);

        for i in start..end {
            if index == i {
                ncurses::wattron(self.win, ncurses::A_REVERSE());
                self.print_file(&dir_contents[i]);
                ncurses::wattroff(self.win, ncurses::A_REVERSE());
            } else {
                self.print_file(&dir_contents[i]);
            }
        }
    }
}

pub struct JoshutoView {
    pub top_win : JoshutoWindow,
    pub left_win : JoshutoWindow,
    pub mid_win : JoshutoWindow,
    pub right_win : JoshutoWindow,
    pub bot_win : JoshutoWindow,
    pub win_ratio : (i32, i32, i32),
}

impl JoshutoView {
    pub fn new(win_ratio : (i32, i32, i32)) -> JoshutoView
    {
        let mut term_rows : i32 = 0;
        let mut term_cols : i32 = 0;
        ncurses::getmaxyx(ncurses::stdscr(), &mut term_rows, &mut term_cols);

        let term_divide : i32 = term_cols / 7;
        let top_win = JoshutoWindow::new(1, term_cols, (0, 0));
        ncurses::scrollok(top_win.win, true);

        let left_win = JoshutoWindow::new(term_rows - 2,
            term_divide * win_ratio.0, (1, 0));

        let mid_win = JoshutoWindow::new(term_rows - 2,
            term_divide * win_ratio.1, (1, term_divide * win_ratio.0));

        let right_win = JoshutoWindow::new(term_rows - 2,
            term_divide * 3, (1, term_divide * win_ratio.2));
        let bot_win = JoshutoWindow::new(1, term_cols, (term_rows - 1, 0));

        ncurses::refresh();

        JoshutoView {
            top_win,
            left_win,
            mid_win,
            right_win,
            bot_win,
            win_ratio,
        }
    }

    pub fn redraw_views(&mut self) {
        let mut term_rows : i32 = 0;
        let mut term_cols : i32 = 0;
        ncurses::getmaxyx(ncurses::stdscr(), &mut term_rows, &mut term_cols);

        let term_divide : i32 = term_cols / 7;

        self.top_win.redraw(1, term_cols, (0, 0));
        ncurses::scrollok(self.top_win.win, true);

        self.left_win.redraw(term_rows - 2,
            term_divide * self.win_ratio.0, (1, 0));

        self.mid_win.redraw(term_rows - 2,
            term_divide * self.win_ratio.1, (1, term_divide * self.win_ratio.0));

        self.right_win.redraw(term_rows - 2,
            term_divide * 3, (1, term_divide * self.win_ratio.2));
        self.bot_win.redraw(1, term_cols, (term_rows - 1, 0));
    }

    pub fn commit(&self)
    {
        self.top_win.commit();
        self.left_win.commit();
        self.mid_win.commit();
        self.right_win.commit();
        self.bot_win.commit();
        ncurses::refresh();
    }
}

fn file_attr_apply(win : ncurses::WINDOW, mode : u32,
        file_extension : Option<&ffi::OsStr>,
        func : fn(ncurses::WINDOW, ncurses::NCURSES_ATTR_T) -> i32)
{
    match mode & unix::BITMASK {
        unix::S_IFDIR => {
            func(win, ncurses::A_BOLD());
            func(win, ncurses::COLOR_PAIR(joshuto::DIR_COLOR));
        },
        unix::S_IFLNK | unix::S_IFCHR | unix::S_IFBLK
         => {
            func(win, ncurses::A_BOLD());
            func(win, ncurses::COLOR_PAIR(joshuto::SOCK_COLOR));
        },
        unix::S_IFSOCK | unix::S_IFIFO => {
            func(win, ncurses::A_BOLD());
            func(win, ncurses::COLOR_PAIR(joshuto::SOCK_COLOR));
        },
        unix::S_IFREG => {
            if unix::is_executable(mode) == true {
                func(win, ncurses::A_BOLD());
                func(win, ncurses::COLOR_PAIR(joshuto::EXEC_COLOR));
            }
            else if let Some(extension) = file_extension {
                if let Some(ext) = extension.to_str() {
                    file_ext_attr_apply(win, ext, func);
                }
            }
        },
        _ => {},
    };
}

fn file_ext_attr_apply(win : ncurses::WINDOW, ext : &str,
        func : fn(ncurses::WINDOW, ncurses::NCURSES_ATTR_T) -> i32)
{
    match ext {
        "png" | "jpg" | "jpeg" | "gif" => {
            func(win, ncurses::COLOR_PAIR(joshuto::IMG_COLOR));
        },
        "mkv" | "mp4" | "mp3" | "flac" | "ogg" | "avi" | "wmv" | "wav" |
        "m4a" => {
            func(win, ncurses::COLOR_PAIR(joshuto::VID_COLOR));
        },
        _ => {},
    }
}