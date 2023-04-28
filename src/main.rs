use clap::{Args, Parser, Subcommand};
use nom::{
    bytes::complete::{tag, take},
    sequence::{separated_pair, terminated},
    IResult,
};
use std::{error::Error, path::PathBuf};
use tokio::{io::BufReader, net::TcpStream};

#[derive(Debug, Clone, Args)]
pub struct Pixel {
    x: u16,
    y: u16,
    color: String,
}

#[derive(Debug, Args)]
pub struct Rect {
    start_x: u16,
    start_y: u16,
    end_x: u16,
    end_y: u16,
    color: String,
}

#[derive(Parser, Debug)]
pub struct Arguments {
    #[command(subcommand)]
    command: Command,

    /// domain of target server
    #[arg(short, long)]
    domain: String,

    // /// how many threads should be used
    // #[arg(short, long)]
    // threads: usize,
    /// should the programm loop indefinetly
    #[arg(short, long)]
    loops: bool,
}

#[derive(Debug, Args)]
pub struct Image {
    x: u16,
    y: u16,
    path: PathBuf,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Howto,
    Pixel(Pixel),
    Rect(Rect),
    Size,
    Image(Image),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Arguments::parse();

    let mut streams = vec![];
    for _ in 0..1 {
        //TODO based on number of threads
        streams.push(BufReader::new(TcpStream::connect(&args.domain).await?));
    }

    match args.command {
        Command::Howto => howto(&mut streams[0]).await?,
        Command::Pixel(pxl) => pixel(&mut streams[0], &pxl).await?,
        Command::Rect(rct) => rect(args.loops, &mut streams[0], rct).await?,
        Command::Size => {
            let size = size(&mut streams[0]).await?;
            println!("{size:?}");
        }
        Command::Image(img) => image(args.loops, &mut streams, img).await?,
    };

    Ok(())
}

async fn image(
    loops: bool,
    stream: &mut [BufReader<TcpStream>],
    img: Image,
) -> Result<(), Box<dyn Error>> {
    let image = image::open(img.path)?.to_rgb8();
    let canvas_limit = size(&mut stream[0]).await?;
    if image.width() > canvas_limit.0 || image.height() > canvas_limit.1 {
        println!("WARN: the image is over the canvas size");
    }

    while loops {
        for pxl in image.enumerate_pixels() {
            if pxl.0 < canvas_limit.0 && pxl.1 < canvas_limit.1 {
                let pxl = Pixel {
                    x: pxl.0 as u16,
                    y: pxl.1 as u16,
                    color: format!(
                        "{:02x?}{:02x?}{:02x?}",
                        pxl.2 .0[0], pxl.2 .0[1], pxl.2 .0[2]
                    ),
                };
                pixel(&mut stream[0], &pxl).await?;
            }
        }
    }

    Ok(())
}

async fn pixel(stream: &mut BufReader<TcpStream>, pxl: &Pixel) -> Result<(), Box<dyn Error>> {
    use tokio::io::AsyncWriteExt;

    let s = format!("PX {} {} {}\n", pxl.x, pxl.y, pxl.color);
    stream.write_all(s.as_bytes()).await?;
    // println!("{s}");
    Ok(())
}

async fn rect(
    loops: bool,
    stream: &mut BufReader<TcpStream>,
    rect: Rect,
) -> Result<(), Box<dyn Error>> {
    use tokio::io::AsyncWriteExt;

    while loops {
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

async fn howto(stream: &mut BufReader<TcpStream>) -> Result<(), Box<dyn Error>> {
    use tokio::io::AsyncBufReadExt;
    use tokio::io::AsyncWriteExt;

    // send HELP
    stream.write_all(b"HELP\n").await?;

    // receive
    let mut buffer = String::with_capacity(256);
    stream.read_line(&mut buffer).await?;
    println!("{buffer:?}");
    Ok(())
}

/// query the size of the pixelflut server canvas
async fn size(stream: &mut BufReader<TcpStream>) -> Result<(u32, u32), Box<dyn Error>> {
    use tokio::io::AsyncBufReadExt;
    use tokio::io::AsyncWriteExt;

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
