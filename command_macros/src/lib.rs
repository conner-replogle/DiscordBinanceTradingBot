
use syn::__private::TokenStream;

#[proc_macro]
pub fn command_path(input: TokenStream) -> TokenStream {
  // 1. Use syn to parse the input tokens into a syntax tree.
  // 2. Use quote to generate new tokens based on what we parsed.
  // 3. Return the generated tokens.
    let paths = std::fs::read_dir("./").unwrap();
    for path in paths {
        println!("Name: {}", path.unwrap().path().display())
    }
    input
}

