use std;
use std::collections::HashMap;
use std::path;

use joshuto::structs;
use joshuto::sort;
use std::collections::hash_map::Entry;

pub struct DirHistory {
    map: HashMap<path::PathBuf, structs::JoshutoDirList>,
}

impl DirHistory {

    pub fn new() -> Self
    {
        DirHistory {
            map: HashMap::new()
        }
    }

    pub fn populate_to_root(&mut self, pathbuf: &path::PathBuf,
       sort_type: &sort::SortType)
    {
        let mut pathbuf = pathbuf.clone();

        while pathbuf.parent() != None {
            {
                let parent = pathbuf.parent().unwrap().to_path_buf();
                match structs::JoshutoDirList::new(parent.clone(), sort_type) {
                    Ok(mut s) => {
                        for (i, dirent) in s.contents.iter().enumerate() {
                            if dirent.path == pathbuf {
                                s.index = i as i32;
                                break;
                            }
                        }
                        self.map.insert(parent, s);
                    },
                    Err(e) => eprintln!("{}", e),
                };
            }
            if pathbuf.pop() == false {
                break;
            }
        }
    }

    pub fn pop_or_create(&mut self, path: &path::Path, sort_type: &sort::SortType)
            -> Result<structs::JoshutoDirList, std::io::Error>
    {
        match self.map.remove(&path.to_path_buf()) {
            Some(mut dir_entry) => {
                if dir_entry.need_update() {
                    dir_entry.update(&sort_type);
                }
                Ok(dir_entry)
            },
            None => {
                structs::JoshutoDirList::new(path.clone().to_path_buf(), &sort_type)
            }
        }
    }

    pub fn get_mut_or_create(&mut self, path: &path::Path,
       sort_type: &sort::SortType)
            -> Option<&mut structs::JoshutoDirList>
    {
        let pathbuf = path.to_path_buf();

        {
            let entry = self.map.entry(pathbuf.clone());
            match entry {
                Entry::Occupied(mut entry) => {
                    let dir_entry = entry.get_mut();
                    if dir_entry.need_update() {
                        dir_entry.update(sort_type);
                    }
                },
                Entry::Vacant(entry) => {
                    if let Ok(s) = structs::JoshutoDirList::new(
                                path.clone().to_path_buf(), &sort_type) {
                        entry.insert(s);
                    }
                },
            };
        }

        self.map.get_mut(&pathbuf)
    }

    pub fn put_back(&mut self, dirlist: Option<structs::JoshutoDirList>)
    {
        if let Some(s) = dirlist {
            self.map.insert(s.path.clone(), s);
        }
    }

    pub fn depecrate_all_entries(&mut self)
    {
        self.map.iter_mut().for_each(|(_, v)| v.update_needed = true);
    }
}
