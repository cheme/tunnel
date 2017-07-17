use readwrite_comp::{
  MultiW,
  MultiWExt,
  MultiRExt,
  ExtRead,
  ExtWrite,
  CompW,
  CompWState,
  CompR,
  CompRState,
  CompExtW,
  CompExtWInner,
  CompExtR,
  CompExtRInner,
};
use super::{
  TunnelWriter,
  Info,
};
use super::common::{
  TunnelState,
};
use std::io::{
  Write,
  Read,
  Result,
};
use mydht_base::peer::Peer;


/**
 * No impl for instance when no error or no reply
 */
pub struct Last ();
/**
 * No impl for instance when no error or no reply
 */
pub struct LastW {
  pub state: TunnelState,
}

pub struct LastR {
}

pub struct LastSRW {
}


impl ExtWrite for Last {
  #[inline]
  fn write_header<W : Write>(&mut self, _ : &mut W) -> Result<()> {
    Ok(())
  }

  #[inline]
  fn write_into<W : Write>(&mut self, _ : &mut W, _ : &[u8]) -> Result<usize> {
    Ok(0)
  }

  #[inline]
  fn write_all_into<W : Write>(&mut self, _ : &mut W, _ : &[u8]) -> Result<()> {
    Ok(())
  }

  #[inline]
  fn flush_into<W : Write>(&mut self, _ : &mut W) -> Result<()> {
    Ok(())
  }

  #[inline]
  fn write_end<W : Write>(&mut self, _ : &mut W) -> Result<()> {
    Ok(())
  }
}

impl ExtRead for Last {
  #[inline]
  fn read_header<R : Read>(&mut self, _ : &mut R) -> Result<()> {
    Ok(())
  }

  #[inline]
  fn read_from<R : Read>(&mut self, _ : &mut R, _ : &mut[u8]) -> Result<usize> {
    Ok(0)
  }

  #[inline]
  fn read_exact_from<R : Read>(&mut self, _ : &mut R, _ : &mut[u8]) -> Result<()> {
    Ok(())
  }

  #[inline]
  fn read_end<R : Read>(&mut self, _ : &mut R) -> Result<()> {
    Ok(())
  }
}


impl TunnelWriter for Last {

}



