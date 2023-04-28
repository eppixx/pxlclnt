use clap::{Args, Parser, Subcommand};
use nom::{
    bytes::complete::{tag, take},
    sequence::{separated_pair, terminated},
    IResult,
};
use std::{
    error::Error,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tokio::{io::BufReader, net::TcpStream};

#[derive(Debug, Clone, Args)]
pub struct Pixel {
    x: u32,
    y: u32,
    color: String,
}

#[derive(Debug, Clone, Args)]
pub struct Rect {
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    color: String,
}

#[derive(Debug, Clone, Args)]
pub struct Image {
    x: u32,
    y: u32,
    path: PathBuf,
}

#[derive(Parser, Clone, Debug)]
pub struct Arguments {
    #[command(subcommand)]
    command: Command,

    /// domain of target server
    #[arg(short, long)]
    domain: String,

    /// how many threads should be used
    #[arg(short, long)]
    threads: usize,

    /// should the programm loop indefinetly
    #[arg(short, long)]
    loops: bool,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    Howto,
    Size,
    Pixel(Pixel),
    Rect(Rect),
    Image(Image),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Arguments::parse();

    match args.command {
        Command::Howto => howto(&args).await?,
        Command::Size => {
            let size = size(&args).await?;
            println!("{size:?}");
        }
        Command::Pixel(pxl) => {
            let mut stream = BufReader::new(TcpStream::connect(&args.domain).await?);
            pixel(&mut stream, &pxl).await?;
        }
        Command::Rect(ref rct) => rect(&args, rct).await?,
        Command::Image(ref img) => image(&args, img).await?,
    };

    Ok(())
}

async fn image(args: &Arguments, img: &Image) -> Result<(), Box<dyn Error>> {
    let image = image::open(&img.path)?.to_rgb8();
    let canvas_limit = size(args).await?;
    if image.width() > canvas_limit.0 || image.height() > canvas_limit.1 {
        println!("WARN: the image is over the canvas size");
    }

    // collect all pixels of image
    let all_pixels: Vec<Pixel> = image
        .enumerate_pixels()
        .map(|pxl| Pixel {
            x: img.x + pxl.0,
            y: img.y + pxl.1,
            color: format!(
                "{:02x?}{:02x?}{:02x?}",
                pxl.2 .0[0], pxl.2 .0[1], pxl.2 .0[2]
            ),
        })
        .collect();

    // divide pixels for threads
    let tasks: Arc<RwLock<Vec<Vec<Pixel>>>> = Arc::new(RwLock::new(vec![]));
    let span = all_pixels.len() / args.threads;
    for i in 0..args.threads {
        let pxls = &all_pixels[(span * i)..(span * (i + 1))];
        let pxls: Vec<Pixel> = pxls
            .iter()
            .filter(|pxl| pxl.x < canvas_limit.0 && pxl.y < canvas_limit.1)
            .cloned()
            .collect();
        tasks.write().unwrap().push(pxls);
    }

    async fn work(loops: bool, task: &Vec<Pixel>) {
        let mut stream = BufReader::new(TcpStream::connect("localhost:1337").await.unwrap());
        while loops {
            for pxl in task {
                pixel(&mut stream, pxl).await.unwrap();
            }
        }
    }

    // spawn threads that work on pixels
    let mut handles = vec![];
    for i in 0..args.threads {
        let task = tasks.clone();
        let loops = args.loops;
        let handle = std::thread::spawn(move || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let task = task.read().unwrap()[i].clone();
                    work(loops, &task).await;
                })
        });
        handles.push(handle);
    }
    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}

/// renders a single pixel
#[inline(always)]
async fn pixel(stream: &mut BufReader<TcpStream>, pxl: &Pixel) -> Result<(), Box<dyn Error>> {
    use tokio::io::AsyncWriteExt;

    // pushing to String is slightly faster that format!()
    let mut s = String::with_capacity(32);
    s.push_str("PX ");
    s.push_str(&pxl.x.to_string());
    s.push(' ');
    s.push_str(&pxl.y.to_string());
    s.push(' ');
    s.push_str(&pxl.color);
    s.push('\n');
    stream.write_all(s.as_bytes()).await?;
    Ok(())
}

/// renders a simple rect single threaded
async fn rect(args: &Arguments, rect: &Rect) -> Result<(), Box<dyn Error>> {
    use tokio::io::AsyncWriteExt;

    let mut stream = BufReader::new(TcpStream::connect(&args.domain).await?);

    while args.loops {
        let pixel = String::from("PX ");
        for x in rect.start_x..rect.end_x {
            for y in rect.start_y..rect.end_y {
                let mut s = pixel.clone();
                s.push_str(&x.to_string());
                s.push(' ');
                s.push_str(&y.to_string());
                s.push(' ');
                s.push_str(&rect.color);
                s.push('\n');
                stream.write_all(s.as_bytes()).await?;
            }
        }
    }
    Ok(())
}

/// prints the HELP command to the pixelflut server
async fn howto(args: &Arguments) -> Result<(), Box<dyn Error>> {
    use tokio::io::AsyncBufReadExt;
    use tokio::io::AsyncWriteExt;

    let mut stream = BufReader::new(TcpStream::connect(&args.domain).await?);

    // send HELP
    stream.write_all(b"HELP\n").await?;

    // receive
    let mut buffer = String::with_capacity(256);
    stream.read_line(&mut buffer).await?;
    println!("{buffer:?}");
    Ok(())
}

/// query the size of the pixelflut server canvas
async fn size(args: &Arguments) -> Result<(u32, u32), Box<dyn Error>> {
    use tokio::io::AsyncBufReadExt;
    use tokio::io::AsyncWriteExt;

    let mut stream = BufReader::new(TcpStream::connect(&args.domain).await?);

    // send SIZE
    stream.write_all(b"SIZE\n").await?;

    // receive
    let mut buffer = String::with_capacity(32);
    stream.read_line(&mut buffer).await?;

    fn parse(input: &str) -> IResult<&str, (u32, u32)> {
        let (rest, _) = take(5u8)(input)?;
        let (rest, parsed) = separated_pair(
            nom::character::complete::u32,
            tag(" "),
            terminated(nom::character::complete::u32, tag("\r\n")),
        )(rest)?;
        Ok((rest, parsed))
    }

    let size = parse(&buffer).unwrap().1;
    Ok(size)
}
