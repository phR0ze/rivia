use rivia::prelude::*;

fn main()
{
    // Write data to a file and read it back using Stdfs and Memfs via a single
    // vfs replacemnent
    let vfs = Vfs::new_stdfs();

    println!("hello");    

    Stdfs::touch("LICENSE-MIT");
}