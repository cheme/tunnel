//! TODO ErrorInfo and ReplyInfo abstraction useless??
//! TODO review all unwrap as most should only return an error, and program could fail very easilly
#![feature(associated_consts)] 
extern crate readwrite_comp;
extern crate rustc_serialize;
extern crate mydht_base;
extern crate bincode;
extern crate rand;
use rustc_serialize::{Encodable, Decodable};
use std::fmt;
use std::cell::BorrowMutError;
use std::cell::BorrowError;
use std::marker::PhantomData;
use rand::thread_rng;
use rand::Rng;
use bincode::SizeLimit;
use mydht_base::transport::Address;
use bincode::rustc_serialize::{
  encode_into as bin_encode, 
  decode_from as bin_decode,
};
use bincode::rustc_serialize::{
  encode,
  decode,
};
use std::convert::Into;
use mydht_base::peer::Peer;
use mydht_base::peer::{
  Shadow,
  ShadowSim,
  ShadowReadOnce,
  ShadowWriteOnce,
  ShadowWriteOnceL,
  new_shadow_read_once,
  new_shadow_write_once,
};
use std::io::{
  Cursor,
  Write,
  Read,
  Result,
  Error as IoError,
  ErrorKind as IoErrorKind,
};
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

use mydht_base::mydhtresult::Error;
use std::slice::Iter;
//use std::marker::Reflect;
use bincode::rustc_serialize::EncodingError as BincError;
use bincode::rustc_serialize::DecodingError as BindError;


pub mod info;
pub mod default;
pub mod nope;
pub mod common;
pub mod full;
pub mod last;
#[cfg(test)]
pub mod tests;

/// Required payload to communicate : to reply or return error
pub trait Info : Sized {
  /// for each peer info to proxy
  fn write_in_header<W : Write>(&mut self, w : &mut W) -> Result<()>;
  /// info to open it for dest if not cached
  fn write_read_info<W : Write>(&mut self, w : &mut W) -> Result<()>;
  /// for each peer read from header
  fn read_from_header<R : Read>(r : &mut R) -> Result<Self>;
  fn read_read_info<R : Read>(&mut self, r : &mut R) -> Result<()>;
  /// if it retun true, there is a need to cache
  /// this info for reply or error routing
  /// If we need cache, reply info must be kept by using a reply key
  fn do_cache (&self) -> bool;

}

pub trait RepInfo : Info {
  fn get_reply_key(&self) -> Option<&Vec<u8>>;
  /// additional writing after content (clear bytes), related to routing scheme
  fn require_additional_payload(&self) -> bool;
}

/// Route Provider
/// P peer for route
/// EI error info
/// RI reply info
/// Routes are always full with origin and dest
pub trait RouteProvider<P : Peer> {
  /// only dest is used to create new route TODO new scheme later with multi dest 
  fn new_route (&mut self, &P) -> Vec<&P>;
  /// for bitunnel (arg is still dest our peer address is known to route provider) 
  /// TODO seems useless : rev previous and in bitunnel a route provider is include
  fn new_reply_route (&mut self, &P) -> Vec<&P>;
}
/// Error info vec do not contain origin (start at index one of route)
pub trait ErrorProvider<P : Peer, EI : Info> {
  /// Error infos bases for peers
  fn new_error_route (&mut self, &[&P]) -> Vec<EI>;
}
/// Reply info vec do not contain origin (start at index one of route)
pub trait ReplyProvider<P : Peer, RI : RepInfo,SSW,SSR> : SymProvider<SSW,SSR,P> {
  /// reply info for dest (last in vec) is different from hop reply info : TODO add new associated type (cf
  /// RepInfo) to avoid mandatory enum on RI.
  /// Last param is dest symetric key to use for reply (could change)
  fn new_reply (&mut self, &[&P]) -> Vec<RI>;
  //fn new_reply (&mut self, &[&P]) -> (Vec<RI>,RI::replypayload); // reply payload as optional in
  //tunnel shadow instead of Replyinfo in tunnelshadowW
}

/// ExtWrite implementation of a TunnelWriter
pub trait TunnelWriterExt : ExtWrite {

  // info only for dest (if needed to read)
  fn write_dest_info<W : Write>(&mut self, w : &mut W) -> Result<()>;
  fn write_dest_info_before<W : Write>(&mut self, w : &mut W) -> Result<()>;
}

// TODO trait seems really useless : change to ExtRead directly??
pub trait TunnelReaderExt : ExtRead {
  type TR; 
  /// retrieve original inner writer
  fn get_reader(self) -> Self::TR;
}

/// Tunnel trait could be in a single tunnel impl, but we use multiple to separate concerns a bit
/// When a tunnel implement multiple trait it has property of all trait ('new_writer' of a
/// TunnelNoRep if Tunnel trait is also implemented will write necessary info for reply).
pub trait TunnelNoRep {
  /// Peer with their address and their asym shadow scheme
  type P : Peer;
  /// actual writer (tunnel logic using tunnel writer)
  type W : TunnelWriterExt;
  // reader must read all three kind of message
  type TR : TunnelReaderNoRep;

  /// TunnelProxy
  type PW : TunnelWriterExt + TunnelReaderExt<TR=Self::TR>;
  /// Dest reader
  type DR : TunnelReaderExt<TR=Self::TR>;
  /// could return a writer allowing reply but not mandatory
  /// same for sym info , param is peer from which we read (used in cacheroute)
  fn new_reader (&mut self, &<Self::P as Peer>::Address) -> Self::TR;
  /// try to init dest (use for cache info)
  fn init_dest(&mut self, &mut Self::TR) -> Result<()>;
  // return writer and next peer
  fn new_writer (&mut self, &Self::P) -> (Self::W, <Self::P as Peer>::Address);
  // TODO rewrite with Iterator next peer is first of roote
  fn new_writer_with_route (&mut self, &[&Self::P]) -> Self::W;
  fn new_proxy_writer (&mut self, Self::TR) -> Result<(Self::PW, <Self::P as Peer>::Address)>;
  fn new_dest_reader<R : Read> (&mut self, Self::TR, &mut R) -> Result<Self::DR>;

}

/// tunnel with reply
pub trait Tunnel : TunnelNoRep where Self::TR : TunnelReader<RI=Self::RI> {
  // reply info info needed to established conn -> TODO type reply info looks useless : we create reply
  // writer from reader which contains it
  type RI : Info; // RI and EI in TunnelError seems useless in this trait except pfor tunnelreader
  type RW : TunnelWriterExt;
  fn new_reply_writer<R : Read> (&mut self, &mut Self::DR, &mut R) -> Result<(Self::RW, <Self::P as Peer>::Address)>;
  // TODO move in reply writer for shorter need of tunnel ?
  fn reply_writer_init<R : Read, W : Write> (&mut self, &mut Self::RW, &mut Self::DR, &mut R, &mut W) -> Result<()>;
}

/// tunnel with reply
pub trait TunnelError : TunnelNoRep where Self::TR : TunnelReaderError<EI=Self::EI> {
  // error info info needed to established conn
  type EI : Info;
  type EW : TunnelErrorWriter; // not an extwrite (use reply writer instead if need : error is lighter and content should be include in error writer

  fn new_error_writer (&mut self, &mut Self::TR) -> Result<(Self::EW, <Self::P as Peer>::Address)>;
  fn proxy_error_writer (&mut self, &mut Self::TR) -> Result<(Self::EW, <Self::P as Peer>::Address)>;
  /// return error peer ix or error (or 0) if unresolved
  fn read_error(&mut self, &mut Self::TR) -> Result<usize>;
}

pub trait CacheIdProducer {
  fn new_cache_id (&mut self) -> Vec<u8>;
}

/// Tunnel which allow caching, and thus establishing connections
/// TODO understand need for where condition
pub trait TunnelManager : Tunnel + CacheIdProducer where Self::RI : RepInfo,
Self::TR : TunnelReader<RI=Self::RI>
{
  // Shadow Sym (if established con)
  type SSCW : ExtWrite;
  // Shadow Sym (if established con) aka destread
  type SSCR : ExtRead;

  fn put_symw(&mut self, &[u8], Self::SSCW, <Self::P as Peer>::Address) -> Result<()>;

  fn get_symw(&mut self, &[u8]) -> Result<(Self::SSCW,<Self::P as Peer>::Address)>;

  fn put_symr(&mut self, Self::SSCR) -> Result<Vec<u8>>;

  fn get_symr(&mut self, &[u8]) -> Result<Self::SSCR>;


  fn use_sym_exchange (&Self::RI) -> bool;

  // first vec is sym key, second is cache id of previous peer
  fn new_sym_writer (&mut self, Vec<u8>, Vec<u8>) -> Self::SSCW;

  fn new_dest_sym_reader (&mut self, Vec<Vec<u8>>) -> Self::SSCR;

}

// TODO need for where ??
pub trait TunnelManagerError : TunnelError + CacheIdProducer where  Self::EI : Info,
Self::TR : TunnelReaderError<EI = Self::EI>
{

  fn put_errw(&mut self, &[u8], Self::EW, <Self::P as Peer>::Address) -> Result<()>;

  fn get_errw(&mut self, &[u8]) -> Result<(Self::EW,<Self::P as Peer>::Address)>;

  fn put_errr(&mut self, &[u8], Vec<Self::EI>) -> Result<()>;

  fn get_errr(&mut self, &[u8]) -> Result<&[Self::EI]>;

}
 
/// Error is for non reply non cache message with only a usize info.
/// Otherwhise Reply mechanism should be use for ack or error
pub trait TunnelErrorWriter {

  fn write_error<W : Write>(&mut self, &mut W) -> Result<()>;

}


/// TODO some fn might be useless : check it later
/// TODO rename to tunnel serializer??
/// Frame writing implementation of a tunnel writer 
/// It could be replaced by simple ExtWrite, but smaller methods are used
pub trait TunnelWriter {
  
/*  /// write state when state is needed 
  fn write_state<W : Write>(&mut self, &mut W) -> Result<()>;

  /// write connection info, currently use for caching of previous peer connection id (no encrypt
  /// on it). This is done at a between peers level (independant to tunnel)
  /// This is not shadowed, but  TODO consider fusion with write_state
  fn write_connect_info<W : Write>(&mut self, &mut W) -> Result<()>;

  /// write headers (probably layered one), and infos (RI, EI, PI) 
  fn write_tunnel_header<W : Write>(&mut self, w : &mut W) -> Result<()>;

  /// write into the tunnel (for dest only)
  fn write_dest_info<W : Write>(&mut self, &mut W) -> Result<()>; 

  /// ExtWrite write into
  fn write_tunnel_into<W : Write>(&mut self, &mut W, &[u8]) -> Result<usize>;

  /// ExtWrite write all into
  fn write_tunnel_all_into<W : Write>(&mut self, &mut W, &[u8]) -> Result<()>;

  /// ExtWrite flush into
  fn flush_tunnel_into<W : Write>(&mut self, _ : &mut W) -> Result<()>;

  /// ExtWrite write end
  fn write_tunnel_end<W : Write>(&mut self, w : &mut W) -> Result<()>;
*/ 
}


pub trait TunnelReaderNoRep : ExtRead {


  fn is_dest(&self) -> Option<bool>; 
  fn is_err(&self) -> Option<bool>; 
/*  fn read_state<R : Read> (&mut self, r : &mut R) -> Result<()>;
  fn read_connect_info<R : Read>(&mut self, &mut R) -> Result<()>;
  fn read_tunnel_header<R : Read>(&mut self, &mut R) -> Result<()>;
  fn read_dest_info<R : Read>(&mut self, &mut R) -> Result<()>;*/
}

pub trait TunnelReaderError : TunnelReaderNoRep {
  type EI;
  fn get_current_error_info(&self) -> Option<&Self::EI>;
}

pub trait TunnelReader : TunnelReaderNoRep {
  type RI;
  fn get_current_reply_info(&self) -> Option<&Self::RI>;
}



/* ??? useless???
pub type TunnelWriterComp<
  'a, 
  'b, 
//  E : ExtWrite + 'b, 
//  P : Peer + 'b, 
//  RI : Info + 'b, 
//  EI : Info + 'b, 
  W : 'a + Write,
  //TW : TunnelWriter<E,P,RI,EI> + 'b> 
  TW : ExtWrite + 'b> 
  = CompW<'a,'b,W,TW>;
*/
pub struct BincErr(BincError);
impl From<BincErr> for IoError {
  #[inline]
  fn from(e : BincErr) -> IoError {
    IoError::new(IoErrorKind::Other, e.0)
  }
}
pub struct BorrMutErr(BorrowMutError);
impl From<BorrMutErr> for IoError {
  #[inline]
  fn from(e : BorrMutErr) -> IoError {
    IoError::new(IoErrorKind::Other, e.0)
  }
}
pub struct BorrErr(BorrowError);
impl From<BorrErr> for IoError {
  #[inline]
  fn from(e : BorrErr) -> IoError {
    IoError::new(IoErrorKind::Other, e.0)
  }
}


pub struct BindErr(BindError);
impl From<BindErr> for IoError {
  #[inline]
  fn from(e : BindErr) -> IoError {
    IoError::new(IoErrorKind::Other, e.0)
  }
}
/// Cache for tunnel : mydht cache is not use directly, but it a mydht
/// cache can implement it very straight forwardly (trait is minimal here) except for creating a
/// key
/// IOresult is used but only for the sake of lazyness (TODO)
pub trait TunnelCache<SSW,SSR> {
  fn put_symw_tunnel(&mut self, &[u8], SSW) -> Result<()>;
  fn get_symw_tunnel(&mut self, &[u8]) -> Result<&mut SSW>;
  fn has_symw_tunnel(&mut self, k : &[u8]) -> bool {
    self.get_symw_tunnel(k).is_ok()
  }

  fn put_symr_tunnel(&mut self, SSR) -> Result<Vec<u8>>;
  fn get_symr_tunnel(&mut self, &[u8]) -> Result<&mut SSR>;
  fn has_symr_tunnel(&mut self, k : &[u8]) -> bool {
    self.get_symr_tunnel(k).is_ok()
  }
}
pub struct TunnelCacheC<C1,C2>(C1,C2);
pub trait TunnelCacheErr<EW,EI> {
  fn put_errw_tunnel(&mut self, &[u8], EW) -> Result<()>;
  fn get_errw_tunnel(&mut self, &[u8]) -> Result<&mut EW>;
  fn has_errw_tunnel(&mut self, k : &[u8]) -> bool {
    self.get_errw_tunnel(k).is_ok()
  }

  fn put_errr_tunnel(&mut self, &[u8], Vec<EI>) -> Result<()>;
  fn get_errr_tunnel(&mut self, &[u8]) -> Result<&[EI]>;
  fn has_errr_tunnel(&mut self, k : &[u8]) -> bool {
    self.get_errr_tunnel(k).is_ok()
  }
}



/// TODO move with generic traits from full (should not be tunnel main module component
/// TODO add Peer as param ? old impl got its w/r from peer
pub trait SymProvider<SSW,SSR,P> {
  fn new_sym_key (&mut self, &P) -> Vec<u8>;
  fn new_sym_writer (&mut self, Vec<u8>) -> SSW;
  fn new_sym_reader (&mut self, Vec<u8>) -> SSR;
}

