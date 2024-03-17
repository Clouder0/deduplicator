use clap::Parser;
use ring::digest::{Context, Digest, SHA256};
use std::collections::HashMap;
use std::fs::copy;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::SystemTime;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct MyArgs {
    /// Name of the person to greet
    #[arg(required = true)]
    paths: Vec<PathBuf>,
}

struct FileResult {
    path: PathBuf,
    filename: String,
    ext: String,
    create_time: SystemTime,
}

struct DigestResult<'a> {
    digest: Vec<u8>,
    file: &'a FileResult,
}

fn sha256_digest<R: Read>(mut reader: R) -> core::result::Result<Digest, std::io::Error> {
    let mut context = Context::new(&SHA256);
    let mut buffer = [0; 1024];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }

    Ok(context.finish())
}

fn get_digest(path: &Path) -> core::result::Result<Digest, std::io::Error> {
    let file = std::fs::File::open(path)?;
    sha256_digest(file)
}

fn search_files(res: &mut Vec<FileResult>, path: &Path) {
    if path.is_dir() {
        path.read_dir().unwrap().for_each(|entry| {
            let entry = entry.unwrap();
            search_files(res, &entry.path());
        });
    } else if path.is_file() {
        res.push(FileResult {
            path: path.to_path_buf(),
            ext: path.extension().unwrap().to_str().unwrap().to_string(),
            create_time: path.metadata().unwrap().created().unwrap(),
            filename: path.file_name().unwrap().to_str().unwrap().to_string(),
        });
    }
}

static mut FILES: Vec<FileResult> = Vec::new();
fn main() {
    let args = MyArgs::parse();
    let mut map: HashMap<String, HashMap<Vec<u8>, Vec<&FileResult>>> = HashMap::new();
    unsafe {
        for path in args.paths {
            search_files(&mut FILES, path.as_path());
        }
    }
    unsafe {
        println!("found {} files", FILES.len());
    }
    let count = thread::available_parallelism().unwrap().get();
    if count < 1_usize {
        panic!("no thread available");
    }
    let mut threads = Vec::new();
    let (res_tx, res_rx) = flume::unbounded();
    let (todo_tx, todo_rx) = flume::unbounded();
    unsafe {
        for f in FILES.iter() {
            todo_tx.send(f).unwrap();
            println!("send {}", f.path.to_str().unwrap());
        }
    }
    for _ in 0..count {
        let thread_res_tx = res_tx.clone();
        let thread_todo_rx = todo_rx.clone();
        threads.push(thread::spawn(move || loop {
            let r_file = thread_todo_rx.try_recv();
            if r_file.is_err() {
                println!("thread exit");
                return;
            }
            let f = r_file.unwrap();
            // println!("recv {}", f.path.to_str().unwrap());
            let digest = get_digest(&f.path).unwrap();
            thread_res_tx
                .send(DigestResult {
                    digest: digest.as_ref().to_vec(),
                    file: f,
                })
                .unwrap();
        }));
    }
    drop(res_tx);
    drop(todo_rx);
    let mut idx = 0;
    loop {
        let t = res_rx.recv();
        if t.is_err() {
            break;
        }
        let d = t.unwrap();
        idx += 1;
        unsafe {
            println!("{}/{}", idx, FILES.len());
        }
        // println!("recv digest for {}", d.file.path.to_str().unwrap());
        map.entry(d.file.ext.clone())
            .or_insert(HashMap::new())
            .entry(d.digest)
            .or_insert(Vec::new())
            .push(d.file);
    }
    map.iter_mut().for_each(|(key, value)| {
        println!("\n\nextension {}:", key);
        let mut t: Vec<&FileResult> = value
            .iter_mut()
            .map(|(_, value)| {
                if value.len() > 1 {
                    println!("found duplicate:");
                    value.sort_unstable_by(|a, b| a.filename.cmp(&b.filename));
                    value.iter().for_each(|file| {
                        println!("{}", file.path.to_str().unwrap());
                    });
                    println!("");
                }
                value[0]
            })
            .collect();
        t.sort_unstable_by(|a, b| a.create_time.cmp(&b.create_time));
        t.iter().enumerate().for_each(|(i, file)| {
            println!("{}.{}: {}", i, key, file.path.to_str().unwrap());
            // copy from path to result/i.key
            // use std::fs::copy;
            if copy(file.path.to_str().unwrap(), format!("result/{}.{}", i, key)).is_err() {
                println!("copy failed for {}", file.path.to_str().unwrap());
            }
        });
    });
}
