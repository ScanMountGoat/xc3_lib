# xc3_test
A command line tool for testing parsing and conversion code for all files in an extracted game dump. 

## Usage
Details for failed conversions will be printed to the console. File types can all be enabled at once or enabled individually.  
`cargo run -p xc3_test --release <path to game dump> --all`  
`cargo run -p xc3_test --release <path to game dump> --mxmd --mibl`
`cargo run -p xc3_test --release <path to game dump> --camdo --mtxt`
