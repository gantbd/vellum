pub mod arithmetic {
    pub fn calculate_total(left: i32, right: i32) -> i32 {
        let typed: i32 = "diagnostic witness";
        left + right
    }
}
pub fn fixture_entry() -> i32 {
    arithmetic::calculate_total(20, 22)
}
