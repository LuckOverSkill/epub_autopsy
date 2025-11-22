Rust command line tool vibe """coded""" in Rust to prepare EPUB files to import into interactive AI tools. 

It takes an `.epub` file for input, splits it into plain text chapters based on the book's internal structure, while filtering out chapters shorter than 200 characters (think big header pages that only have the chapter's title).

It spits out the output in your /home/username/Documents folder, creating a `Split_Books/booktitle` folder.

Once you install it via `cargo install --path . --force` you can just type `epub_autopsy /path/to/book.epub` in the terminal and it will do the thingy.

I'm not a dev I just needed this for personal use.
