use wasm_bindgen::prelude::*;     //bringing wasm-bindgen into scope

// Adding this attribute so that wasm-bindgen generates JS glue for this func
#[wasm_bindgen]
pub fn add(left: u32, right: u32) -> u32 {
    left + right
}
// Using u32 so add works with JS.
// Using u32 for "number" on the JS side. 
// Only dealing with small nums, grid values for Alien Defense,
// No need for Rust's u64, which goes higher than JS normal "number"
// If we left u64 as is, then: TypeScript calls add(2n, 3n) with result -> BigInt 

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
