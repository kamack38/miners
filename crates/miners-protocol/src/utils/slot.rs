use crate::utils::nbt::NBTType;

#[derive(Debug, Clone)]
pub struct Slot {
    pub present: bool,
    pub item_id: Option<i32>,
    pub item_count: Option<u8>,
    pub nbt: Option<NBTType>,
}
