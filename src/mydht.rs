extern crate mydht_base;

use super::Peer as TPeer;
use self::mydht_base::peer::Peer as MPeer;

impl<P : MPeer> TPeer for P {
  type Address = P::Address;
  type ShadRead = P::ShadowRMsg;
  type ShadWrite = P::ShadowWMsg;

  fn get_address(&self) -> &Self::Address {
    self.get_address()
  }
  fn new_shadw(&self) -> Self::ShadWrite {
    self.get_shadower_w_msg()
  }
  fn new_shadr(&self) -> Self::ShadRead {
    self.get_shadower_r_msg()
  }

}

