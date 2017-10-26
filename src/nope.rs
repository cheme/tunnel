use std::marker::PhantomData;
use readwrite_comp::{
  ExtRead,
  ExtWrite,
};
use super::{
  TunnelWriter,
  TunnelNoRep,
  TunnelReadProv,
  TunnelWriterExt,
  TunnelReaderExt,
  TunnelCache,
  TunnelCacheErr,
  CacheIdProducer,
  TunnelErrorWriter,
  TunnelReaderNoRep,
  Info,
  RepInfo,
  SymProvider,
  RouteProvider,
  ErrorProvider,
  ReplyProvider,
  Peer,
};
use std::io::{
  Write,
  Read,
  Result,
};


/**
 * No impl for instance when no error or no reply
 */
#[derive(Clone)]
pub struct Nope;


impl Info for Nope {

  #[inline]
  fn do_cache (&self) -> bool {
    false
  }

  #[inline]
  fn write_in_header<W : Write>(&mut self, _ : &mut W) -> Result<()> {
    Ok(())
  }
  #[inline]
  fn write_read_info<W : Write>(&mut self, _ : &mut W) -> Result<()> {
    Ok(())
  }
  #[inline]
  fn read_from_header<R : Read>(_ : &mut R) -> Result<Self> {
    Ok(Nope)
  }
  #[inline]
  fn read_read_info<R : Read>(&mut self, _ : &mut R) -> Result<()> {
    Ok(())
  }

}
impl RepInfo for Nope {

  #[inline]
  fn require_additional_payload(&self) -> bool {
    false
  }
  #[inline]
  fn get_reply_key(&self) -> Option<&Vec<u8>> {
    None
  }
}

impl ExtWrite for Nope {
  #[inline]
  fn write_header<W : Write>(&mut self, _ : &mut W) -> Result<()> {
    Ok(())
  }

  #[inline]
  fn write_into<W : Write>(&mut self, _ : &mut W, c : &[u8]) -> Result<usize> {
    Ok(c.len())
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

impl ExtRead for Nope {
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


impl TunnelWriter for Nope {
}

impl TunnelWriterExt for Nope {
  #[inline]
  fn write_dest_info_before<W : Write>(&mut self, _ : &mut W) -> Result<()> {Ok(())}
  #[inline]
  fn write_dest_info<W : Write>(&mut self, _ : &mut W) -> Result<()> {Ok(())}
}
impl TunnelErrorWriter for Nope {
  fn write_error<W : Write>(&mut self, _ : &mut W) -> Result<()> {
    Ok(())
  }
}
impl TunnelReaderNoRep for Nope {
  #[inline]
  fn is_dest(&self) -> Option<bool> {None}
  #[inline]
  fn is_err(&self) -> Option<bool> {None}

}

impl CacheIdProducer for Nope {
  fn new_cache_id (&mut self) -> Vec<u8> {
    Vec::new()
  }
}

impl<EW,EI> TunnelCacheErr<EW,EI> for Nope {
  fn put_errw_tunnel(&mut self, _ : Vec<u8>, _ : EW) -> Result<()> {
    // TODO replace with actual erro
    unimplemented!()
  }
  fn get_errw_tunnel(&mut self, _ : &Vec<u8>) -> Result<&mut EW> {
    // TODO replace with actual erro
    unimplemented!()
  }
  fn put_errr_tunnel(&mut self, _ : Vec<u8>, _ : Vec<EI>) -> Result<()> {
   // TODO replace with actual erro
    unimplemented!()
  }
  
  fn get_errr_tunnel(&mut self, _ : &Vec<u8>) -> Result<&Vec<EI>> {
   // TODO replace with actual erro
    unimplemented!()
  }
  
 
}
impl<SSW,SSR> TunnelCache<SSW,SSR> for Nope {
  fn put_symw_tunnel(&mut self, _ : Vec<u8>, _ : SSW) -> Result<()> {
    // TODO replace with actual erro
    unimplemented!()
  }
  fn get_symw_tunnel(&mut self, _ : &Vec<u8>) -> Result<&mut SSW> {
    // TODO replace with actual erro
    unimplemented!()
  }
  fn remove_symw_tunnel(&mut self, _ : &Vec<u8>) -> Result<SSW> {
    // TODO replace with actual erro
    unimplemented!()
  }


  fn has_symw_tunnel(&mut self, _ : &Vec<u8>) -> bool {
    false
  }

  fn put_symr_tunnel(&mut self, _ : SSR) -> Result<Vec<u8>> {
    // TODO replace with actual erro
    unimplemented!()
  }
  fn get_symr_tunnel(&mut self, _ : &Vec<u8>) -> Result<&mut SSR> {
    // TODO replace with actual erro
    unimplemented!()
  }
  fn remove_symr_tunnel(&mut self, _ : &Vec<u8>) -> Result<SSR> {
    // TODO replace with actual erro
    unimplemented!()
  }
 
  fn has_symr_tunnel(&mut self, _ : &Vec<u8>) -> bool {
    false
  }
}
// TODO remove as only for dev progress
impl<SSW,SSR> SymProvider<SSW,SSR> for Nope {

  fn new_sym_key (&mut self) -> Vec<u8> {
    panic!("Should only be use for dev");
  }
  fn new_sym_writer (&mut self, _ : Vec<u8>) -> SSW {
    panic!("Should only be use for dev");
  }
  fn new_sym_reader (&mut self, _ : Vec<u8>) -> SSR {
    panic!("Should only be use for dev");
  }

}

impl<P : Peer> RouteProvider<P> for Nope {

  fn new_route (&mut self, _ : &P) -> Vec<&P> {
    panic!("Placeholder, should not be called");
  }
  fn new_reply_route (&mut self, _ : &P) -> Vec<&P> {
    panic!("Placeholder, should not be called");
  }

}

impl<P : Peer, EI : Info> ErrorProvider<P,EI> for Nope {
  fn new_error_route (&mut self, _ : &[&P]) -> Vec<EI> {
    panic!("Placeholder, should not be called");
  }
}
impl<P : Peer, RI : RepInfo> ReplyProvider<P,RI> for Nope {
  fn new_reply (&mut self, _ : &[&P]) -> Vec<RI> {
    panic!("Placeholder, should not be called");
  }
}
impl TunnelReaderExt for Nope {
  type TR = Nope; 
  /// retrieve original inner writer
  fn get_reader(self) -> Self::TR {
    self
  }
}
pub struct TunnelNope<P : Peer> (PhantomData<(P)>);
impl<P : Peer> TunnelNope<P> {
  pub fn new() -> Self {
    TunnelNope(PhantomData)
  }
}
impl<P : Peer> TunnelNoRep for TunnelNope<P> {
  type ReadProv = Self;
  type P = P;
  type W = Nope;
  type TR = Nope;
  type PW = Nope;
  type DR = Nope;
  fn new_reader (&mut self) -> Self::TR { Nope }
  fn init_dest(&mut self, _ : &mut Self::TR) -> Result<()> {Ok(())}
  fn new_writer (&mut self, dest : &Self::P) -> (Self::W, <Self::P as Peer>::Address) {(Nope,dest.get_address().clone())}
  fn new_writer_with_route (&mut self, _ : &[&Self::P]) -> Self::W {Nope}
  fn new_proxy_writer (&mut self, _ : Self::TR, _ : &<Self::P as Peer>::Address) -> Result<(Self::PW,<Self::P as Peer>::Address)> {panic!("Nope do not implement that")}
  fn new_dest_reader<R : Read> (&mut self, _ : Self::TR, _ : &mut R) -> Result<Self::DR> {Ok(Nope)}
  fn new_tunnel_read_prov (&self) -> Self::ReadProv {TunnelNope::new()}
}
impl<P : Peer> TunnelReadProv for TunnelNope<P> {
  type T = Self;
  fn new_reader (&mut self) -> <Self::T as TunnelNoRep>::TR { Nope }
  fn new_dest_reader<R : Read> (&mut self, _ : <Self::T as TunnelNoRep>::TR, _ : &mut R) -> Result<Option<<Self::T as TunnelNoRep>::DR>> { Ok(Some(Nope)) }
}


