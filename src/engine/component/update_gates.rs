use crate::engine::BitMask;

/// Used by Entities which are subject to pausing. If the Game's pause mask matches 
/// the Entity's PauseMask::mask on at least one bit, the Entity will not be updated if Updatable.
/// Meaningless at Engine-level if the owner Entity is not Updatable.
///
/// (Updating may also depend on other update gates)
pub struct PauseMask {
    pub mask: BitMask
}

/// Used by Entities which may be arbitrarily active or not. If ActiveFlag::flag is false, the 
/// Entity will not be updated if Updatable. 
/// Meaningless at Engine-level if the owner Entity is not Updatable.
/// 
/// (Updating may also depend on other update gates)
pub struct ActiveFlag {
    pub flag: bool
}