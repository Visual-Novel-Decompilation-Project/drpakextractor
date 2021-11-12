use std::io::{BufReader, BufRead, SeekFrom, Seek, Read, Write};
use std::fs::File;
use std::{env, fs, thread};
use std::path::{Path, PathBuf};
use std::convert::TryInto;
use tinytga::{ImageType, Pixel, Tga, TgaFooter, TgaHeader};
use std::ffi::OsString;
use std::borrow::Borrow;
use std::any::Any;
use clap::Clap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::error::Error;
use encoding_rs::SHIFT_JIS;
use std::ops::{Deref, Add};

fn main() {
    println!("Danganronpa UnPAKer");
    println!("A mappings file is required to restore filenames and file extensions.");
    println!("Without a mappings file, Auto filetype detection will be used.");
    println!("Auto filetype detection will do it's best, but it can have false-positives.\n\n");

    let mut opts: Opts = Opts::parse();

    if opts.pakFile.is_some() {
        processPakFile(&*opts.pakFile.unwrap(), &opts.textureFolder, &opts.noesisExe, &opts.mapFile, &opts.useFbx, &Option::None);
        println!("All good! Enjoy!");
        return;
    }
    if opts.paksFolder.is_some() {
        let paksDir = fs::read_dir(opts.paksFolder.unwrap()).unwrap();
        let currentDir = env::current_dir().unwrap();

        for file in paksDir {
            if file.unwrap().path().as_path().extension() != "pak" {
                continue;
            }

            let mut folderName = std::env::current_dir().unwrap().to_str().unwrap().to_string() + &"\\extracted_".to_string();
            folderName = folderName + &unwrapped.path().file_stem().unwrap().to_str().unwrap();

            fs::create_dir_all(&currentDir.join(Path::new(&folderName)));
            processPakFile(&unwrapped.path().to_string_lossy(), &opts.textureFolder, &opts.noesisExe, &opts.mapFile, &opts.useFbx, &Some(&folderName));
            env::set_current_dir(&currentDir.as_path());
        }
    }
    println!("All good! Enjoy!");
    return;

    // todo leave noesis output only
    fn processPakFile(pak_file_path: &str, texture_folder_path: &Option<String>, noesis_exe_path: &Option<String>, map_file_path: &Option<String>, use_fbx: &Option<bool>, extraction_folder: &Option<&String>) {

        let mut folderPath;
        let file = pak_file_path;

        if extraction_folder.is_some() {
            folderPath = extraction_folder.unwrap().to_string();
        }
        else {
            folderPath =  String::from("extracted_".to_string().add(&*Path::file_stem(&pak_file_path.as_ref()).unwrap().to_string_lossy()));
        }

        fs::create_dir_all(&folderPath);
        env::set_current_dir(&folderPath);

        println!("Loading {}...", Path::file_stem(&pak_file_path.as_ref()).unwrap().to_string_lossy());
        let mut buf = BufReader::new(File::open(&file).unwrap());
        let mut filestem = Path::file_stem(&file.as_ref()).unwrap();

        buf.fill_buf();

        let mut dst = [0u8; 4];
        dst.copy_from_slice(&buf.buffer()[..4]);
        let fileCount = i32::from_le_bytes(dst);
        buf.seek(SeekFrom::Current(4));
        buf.fill_buf();

        let mut offsets: Vec<i32> = Vec::new();

        // Get the file count
        for i in 0..fileCount {
            dst.copy_from_slice(&buf.buffer()[..4]);
            let offset = i32::from_le_bytes(dst);
            offsets.push(offset);
            buf.seek(SeekFrom::Current(4));
            buf.fill_buf();
        }

        let mut maps;
        let mut map: Option<Vec<String>> = None;
        let mut isMappingUsed = false;

        if map_file_path.is_some() {
            let mut mappingFile = &mut "".to_string();
            File::open(map_file_path.as_ref().unwrap()).unwrap().read_to_string(mappingFile);
            maps = serde_json::from_str::<HashMap<String, Vec<String>>>(mappingFile).unwrap();

            for i in maps {
                if i.0 == Path::file_stem(pak_file_path.as_ref()).unwrap().to_str().unwrap() {
                    map = Some(i.1);
                    break;
                }
            }

            if map.is_none() {
                println!("File {1} was not found in {0}, skipping mapping.", Path::file_stem(map_file_path.as_ref().unwrap().as_ref()).unwrap().to_string_lossy().into_owned(), Path::file_stem(pak_file_path.as_ref()).unwrap().to_string_lossy().into_owned());
            } else {
                println!("Mappings loaded.");
                isMappingUsed = true;
            }
        }

        // Iterate over the offsets
        for i in 0..fileCount {
            buf.seek(SeekFrom::Start(offsets[i as usize] as u64));
            buf.fill_buf();

            // filestem = Path::file_stem(file.as_ref()).unwrap();
            let mut fs = filestem.to_str().unwrap();
            let x = fs.to_owned() + "_" + i.to_string().as_str();

            let pBuf = Path::new(folderPath.as_str()).join(PathBuf::from(&x));
            let extractTo = pBuf.to_str().unwrap();
            let mut data;
            let mut newFile;
            let mut fileName = &"".to_string();

            match map.as_ref() {
                Some(f) => {
                    newFile = File::create(Path::new(folderPath.as_str()).join(Path::new(&f[i as usize]))).unwrap();
                    fileName = &f[i as usize];
                },
                None => newFile = File::create(extractTo).unwrap()
            }

            buf.seek(SeekFrom::Start(offsets[i as usize] as u64));
            buf.fill_buf();

            // Zero-index final offset
            if i == fileCount - 1 {
                let eof = buf.seek(SeekFrom::End(0)).unwrap();
                data = vec![0u8; (&eof - offsets[i as usize] as u64) as usize].into_boxed_slice();

                buf.seek(SeekFrom::Start(offsets[i as usize] as u64));
                buf.fill_buf();

                buf.read_exact(&mut data);
                newFile.write_all(&data);

                if map.is_some() {
                    println!("Finally writing {0} bytes to {1}", &eof - i as u64, fileName);
                } else {
                    println!("Finally writing {0} bytes to {1}", &eof - i as u64, x);
                }
                break;
            }

            let mut offsetCurrent = offsets[i as usize];
            let mut offsetNext = offsets[(i + 1) as usize];

            data = vec![0u8; (offsetNext - offsetCurrent) as usize].into_boxed_slice();
            buf.read_exact(&mut data);
            newFile.write_all(&data);

            if map.is_some() {
                println!("Writing {0} bytes to {1}", offsetNext - offsetCurrent, &fileName);
            } else {
                println!("Writing {0} bytes to {1}", offsetNext - offsetCurrent, x);
            }
        }
        println!("Finished writing bytes. Now identifying files...");

        let mut extracted = fs::read_dir(&folderPath).unwrap();
        let mut alreadyDone: Vec<String> = Vec::new();

        if isMappingUsed {
            println!("Autodetection skipped, using mappings file.");
        } else {
            println!("Autodetecting file extensions, file was not found in mappings file.");

            for x in extracted {
                let mut direntry = x.unwrap();

                if alreadyDone.contains(&direntry.path().to_string_lossy().to_string()) {
                    continue;
                }

                if direntry.file_type().unwrap().is_dir() {
                    continue;
                }

                let mut buf = BufReader::new(File::open(direntry.path()).unwrap());
                buf.fill_buf();

                let gmoMagic = String::from_utf8_lossy(&buf.buffer()[0..11]).into_owned();
                let lffdMagic = String::from_utf8_lossy(&buf.buffer()[0..3]).into_owned();
                let mut rename = direntry.path().clone();

                // Check if we are GMO
                if gmoMagic == "OMG.00.1PSP" {
                    println!("{0} is now {1}", direntry.path().file_name().unwrap().to_str().unwrap(), direntry.path().file_name().unwrap().to_str().unwrap().to_owned() + ".gmo");
                    rename.set_extension("gmo");
                    fs::rename(direntry.path(), &rename);

                    alreadyDone.push(rename.to_string_lossy().to_string());
                    continue;
                }
                // Check if we are LFFD
                if lffdMagic == "LFFD" {
                    println!("{0} is now {1}", direntry.path().file_name().unwrap().to_str().unwrap(), direntry.path().file_name().unwrap().to_str().unwrap().to_owned() + ".lffd");
                    rename.set_extension("lffd");
                    fs::rename(direntry.path(), &rename);
                    alreadyDone.push(rename.to_string_lossy().to_string());
                    continue;
                }

                {
                    let mut read = fs::read(direntry.path()).unwrap();
                    let mut img = Tga::from_slice(&*read);
                    if img.is_ok() {
                        // False positive - no height or width or empty pixel data
                        if img.unwrap().header.height == 0 || img.unwrap().header.width == 0 || img.unwrap().header.image_type == ImageType::Empty {
                            println!("{0} is now {1}", direntry.path().file_name().unwrap().to_str().unwrap(), direntry.path().file_name().unwrap().to_str().unwrap().to_owned() + ".dat");
                            rename.set_extension("dat");
                            fs::rename(direntry.path(), &rename);
                            alreadyDone.push(rename.to_string_lossy().to_string());
                            continue;
                        }

                        // We are a TGA File.
                        println!("{0} is now {1}", direntry.path().file_name().unwrap().to_str().unwrap(), direntry.path().file_name().unwrap().to_str().unwrap().to_owned() + ".tga");
                        rename.set_extension("tga");
                        fs::rename(direntry.path(), &rename);
                        alreadyDone.push(rename.to_string_lossy().to_string());
                        continue;
                    }

                    // We are a file of unknown format.
                    println!("{0} is now {1}", direntry.path().file_name().unwrap().to_str().unwrap(), direntry.path().file_name().unwrap().to_str().unwrap().to_owned() + ".dat");
                    rename.set_extension("dat");
                    fs::rename(direntry.path(), &rename);
                    alreadyDone.push(rename.to_string_lossy().to_string());
                    continue;
                }
            }
        }

        if noesis_exe_path.is_some() {
            //region gmo to gltf
            let mut ext = ".gltf";
            if use_fbx.is_some() {
                if use_fbx.unwrap() {
                    ext = ".fbx"
                }
            }

            println!("Compiling to {} using Noesis...", ext);

            env::set_current_dir(Path::new(&folderPath));
            fs::create_dir_all(env::current_dir().unwrap().join(Path::new("noeout")));

            if texture_folder_path.is_none() {
                println!("Missing localized (translated) texture files, textures will need to be manually imported!");
            } else {
                // create hardlinks
                env::set_current_dir(Path::new(&folderPath.as_str()));

                for tex in fs::read_dir(texture_folder_path.as_ref().unwrap()).unwrap().into_iter() {
                    // hard link because symlinks suck
                    let sym = std::fs::hard_link(tex.as_ref().unwrap().path().as_path(), env::current_dir().unwrap().join(tex.as_ref().unwrap().path().file_name().unwrap()).as_path());

                    if sym.is_err() {
                        // todo: windows specific error code
                        if sym.as_ref().err().unwrap().raw_os_error().unwrap() == 183 {
                            println!("WARN: File already exists, taking DEST:");
                            println!("SOURCE: {0} \nDEST: {1}", tex.as_ref().unwrap().path().as_path().to_string_lossy(), env::current_dir().unwrap().join(tex.as_ref().unwrap().path().file_name().unwrap()).as_path().to_string_lossy());
                            continue;
                        }
                        println!("Please run the app as administrator and try again. Err: Cannot create hardlink for texture");
                        println!("{}", sym.as_ref().err().unwrap());
                        println!("SOURCE: {0} \n DEST: {1}", tex.as_ref().unwrap().path().as_path().to_string_lossy(), env::current_dir().unwrap().join(tex.as_ref().unwrap().path().file_name().unwrap()).as_path().to_string_lossy());
                        fs::remove_dir_all(env::current_dir().unwrap());
                        return;
                    }
                }
            }

            extracted = fs::read_dir(&folderPath).unwrap();
            let mut noesis_threads = vec![];


            for x in extracted {
                let fielanme = &x.as_ref().unwrap().file_name();
                let mut data = Path::new(fielanme);
                let gmoMatch = match data.extension() {
                    None => false,
                    Some(x) => {
                        if x == "gmo" {
                            true
                        } else {
                            false
                        }
                    }
                };

                if gmoMatch {
                    let newfielanme = Path::new("noeout").join(x.as_ref().unwrap().path().file_stem().unwrap().to_str().unwrap().to_owned() + ext);
                    println!("Writing {}", newfielanme.to_string_lossy());

                    let noesisExe = noesis_exe_path.as_ref().unwrap().clone();
                    noesis_threads.push(thread::spawn(move || { Command::new(noesisExe).arg("?cmode").arg(x.unwrap().path().to_str().unwrap()).arg(&newfielanme.to_str().unwrap()).stdout(Stdio::null()).output().expect("Cannot start Noesis instance!"); }));
                }
            }

            for thread in noesis_threads {
                thread.join().unwrap();
            }

            if texture_folder_path.is_some() {
                for tex in fs::read_dir(texture_folder_path.as_ref().unwrap()).unwrap().into_iter() {
                    fs::remove_file(env::current_dir().unwrap().join(tex.unwrap().path().file_name().unwrap()));
                }
            }

            if ext == ".gltf" {
                println!("Fixing known glTF issues...");
                env::set_current_dir(env::current_dir().unwrap().join("noeout"));

                for x in fs::read_dir(std::env::current_dir().unwrap().as_path()).unwrap() {
                    if x.as_ref().unwrap().file_type().unwrap().is_dir() {
                        continue;
                    }


                    let curFile = &x.as_ref().unwrap().file_name();
                    let mut data = Path::new(curFile);
                    let gltfMatch = match data.extension() {
                        None => false,
                        Some(x) => {
                            if x == "gltf" {
                                true
                            } else {
                                false
                            }
                        }
                    };

                    if gltfMatch {
                        let mut str = &mut "".to_string();
                        let mut fileBufreader = File::open(x.as_ref().unwrap().path()).unwrap();

                        // Sometimes there is Shift-JIS content.
                        // This causes a lot of stuff go to awry.
                        let mut decoder = SHIFT_JIS.new_decoder();
                        let mut rawData = &mut vec![];

                        fileBufreader.read_to_end(rawData).unwrap();

                        let decres = SHIFT_JIS.decode(rawData);
                        if decres.2 {
                            println!("{:?},{}", decres.1, decres.2);
                            panic!();
                        }

                        let string = &mut decres.0.into_owned();
                        str = string;

                        // File paths in the GMO can be 'deformed' (i.e using backslash instead of fwdslash for paths) so we need to fix this.
                        // Ideally this would be a fix within Noesis itself.
                        let replaced = str.replace("\\", "/");

                        // Replace colon in file paths with an underscore.
                        // Helps with programs that don't like colons.
                        let secondReplace = replaced.replace(":/", "_/");

                        fs::write(x.as_ref().unwrap().path(), secondReplace).unwrap();
                        println!("Fixed {}", x.as_ref().unwrap().path().file_stem().unwrap().to_string_lossy());
                    }
                }
            }
        }
    }

    #[derive(Clap)]
    #[clap(version = "1.0", author = "breadbyte")]
    struct Opts {
        /// The folder containing PAKs to be parsed.
        #[clap(long)]
        paksFolder: Option<String>,

        /// The PAK File to be parsed.
        #[clap(long)]
        pakFile: Option<String>,

        /// The Extended Textures folder, usually found in region-specific WADs.
        #[clap(long)]
        textureFolder: Option<String>,

        /// Noesis EXE for converting 3D Formats to easily-readable ones.
        #[clap(long)]
        noesisExe: Option<String>,

        /// Mappings file generated by DRPakMapGen.
        /// Or just generated by hand, i don't know.
        #[clap(long)]
        mapFile: Option<String>,

        /// Whether or not to use FBX. This is mostly a DEBUG OPTION. Noesis sometimes has issues with it's exported glTF, or programs have issues importing glTF from Noesis.
        /// FBX is [NOT RECOMMENDED] unless necessary, as it requires extensive time-consuming extra work done that glTF does automatically.
        // I've experimented with multiple output types with Noesis, and glTF is the only one that gives me the optimal results that i want.
        // The time consuming work is taking the time to put the textures in, and having to mix it with the vertex colors.
        // Exporting to glTF does this automagically.
        #[clap(long)]
        useFbx: Option<bool>
    }

    #[derive(Deserialize)]
    struct RootJson {
        map: Vec<Mapping>
    }

    #[derive(Deserialize)]
    struct Mapping {
        filename: String,
        files: Vec<String>
    }
}
