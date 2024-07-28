// use on a PathBuf as follows
// use_path!(root, "file.md", path => {
//  load_path(&path)
// });
#[macro_export]
macro_rules! use_path {
    ($buf: expr, $rest: expr; $as: ident => $blk: tt) => {{
        let $as = &mut $buf;
        $as.push($rest);

        let result = $blk;

        $as.pop();
        result
    }};
}
