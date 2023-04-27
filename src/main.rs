use clap::Parser;
use clap_derive::{Parser, Subcommand};
use nom::{
    bytes::complete::{tag, take},
    multi::separated_list1,
    sequence::{delimited, separated_pair, terminated},
    IResult,
};
use std::{error::Error, fmt::Display};
use tokio::{io::BufReader, net::TcpStream};

#[derive(Debug, Clone)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug)]
pub enum Cmd {
    Help,
    Px { x: u32, y: u32, c: Color },
    Size,
}

#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    command: Command,

    /// domain of target server
    #[arg(short, long)]
    domain: String,

    /// how many threads should be used
    #[arg(short, long)]
    threads: Option<i8>,

    /// size limit of the tcp packets
    #[arg(short, long)]
    size: Option<i16>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Pixel {
        x: i16,
        y: i16,
        color: String,
    },
    Rect {
        start_x: i16,
        start_y: i16,
        end_x: i16,
        end_y: i16,
        color: String,
    },
    Size,
    //Image,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");
    let args = Args::parse();

    let mut streams = vec![BufReader::new(TcpStream::connect(&args.domain).await?)];

    match args.command {
        Command::Pixel { x, y, color } => todo!(),
        Command::Rect {
            start_x,
            start_y,
            end_x,
            end_y,
            color,
        } => todo!(),
        Command::Size => {
            let size = size(&mut streams[0]).await;
            println!("{size:?}");
        }
    };

    Ok(())
}

/// query the size of the pixelflut server canvas
async fn size(stream: &mut BufReader<TcpStream>) -> Result<(i32, i32), Box<dyn Error>> {
    use tokio::io::AsyncBufReadExt;
    use tokio::io::AsyncWriteExt;

    // send SIZE
    stream.write_all(b"SIZE\n\n").await?;

    // receive
    let mut buffer = String::with_capacity(32);
    stream.read_line(&mut buffer).await?;

    fn parse(input: &str) -> IResult<&str, (i32, i32)> {
        let (rest, _) = take(5u8)(input)?;
        let (rest, parsed) = separated_pair(
            nom::character::complete::i32,
            tag(" "),
            terminated(nom::character::complete::i32, tag("\r\n")),
        )(rest)?;
        Ok((rest, parsed))
    }

    let size = parse(&buffer).unwrap().1;
    Ok(size)
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02x?}{:02x?}{:02x?}", self.r, self.g, self.b)
    }
}

impl Display for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::Help => write!(f, "HELP"),
            Cmd::Px { x, y, c } => write!(f, "PX {x} {y} {c}"),
            Cmd::Size => todo!(),
        }
    }
}
