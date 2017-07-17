extern crate mydht_basetest;
use std::rc::Rc;
use std::cmp;
use std::cell::RefCell;
use rand::ThreadRng;
use rand::thread_rng;
use rand::Rng;

use std::io::{Error, ErrorKind};
use mydht_base::peer::{
  Peer,
  Shadow,
  ShadowSim,
};
use std::marker::PhantomData;
use super::{
  TunnelCache,
  TunnelCacheErr,
  RouteProvider,
  SymProvider,
  ReplyProvider,
  TunnelNoRep,
  TunnelReaderNoRep,
  CacheIdProducer,
  Tunnel,
  TunnelError,
  TunnelErrorWriter,
  TunnelReaderExt,
};
use super::nope::{Nope,TunnelNope};
use super::full::{
  ErrorWriter,
  Full,
  DestFull,
  FullR,
  GenTunnelTraits,
  TunnelCachedWriterExt,
  TunnelCachedReaderExt,

  TunnelCachedWriterExtClone,
  TunnelCachedReaderExtClone,
  FullW,
};
use super::info::multi::{
  MultipleReplyMode,
  ReplyInfoProvider,
  MultipleReplyInfo,
  NoMultiRepProvider,
};
use super::info::error::{
  MultipleErrorInfo,
  MultipleErrorMode,
  MulErrorProvider,
  NoErrorProvider,
};
use std::collections::HashMap;
use std::io::{
  Write,
  Read,
  Result,
  Cursor,
};
use rand::os::OsRng;

use readwrite_comp::{
  ExtRead,
  ExtWrite,
  CompW,
  CompR,
};
use self::mydht_basetest::peer::{
  PeerTest,
};
use self::mydht_basetest::transport::{
  LocalAdd,
};
use self::mydht_basetest::shadow::{
  ShadowTest,
  ShadowModeTest,
};
use mydht_base::bytes_wr::sized_windows::{
  SizedWindowsParams,
  SizedWindows,
};

#[derive(Clone)]
pub struct TestSizedWindows;

#[derive(Clone)]
pub struct TestSizedWindowsHead;
impl SizedWindowsParams for TestSizedWindowsHead {
    const INIT_SIZE : usize = 15;
    const GROWTH_RATIO : Option<(usize,usize)> = None;
    const WRITE_SIZE : bool = false;
    const SECURE_PAD : bool = false;
}

impl SizedWindowsParams for TestSizedWindows {
//    const INIT_SIZE : usize = 45;
    const INIT_SIZE : usize = 15;
    const GROWTH_RATIO : Option<(usize,usize)> = Some((3,2));
    const WRITE_SIZE : bool = true;
    const SECURE_PAD : bool = false;
}

type CachedR = TunnelCachedReaderExtClone<SRead>;
type CachedW = TunnelCachedWriterExtClone<SWrite,SizedWindows<TestSizedWindows>>;

pub struct CachedInfo<P : Peer> {
  // TODO rename field plus rem option
  pub cached_key : Option<(CachedW,<P as Peer>::Address)>,
  pub prev_peer : Vec<u8>,// TODO rename that is key!!!
}
pub struct CachedInfoE<P : Peer> {
  // TODO rename field plus rem option
  pub cached_key : Option<(ErrorWriter,<P as Peer>::Address)>,
  pub prev_peer : Vec<u8>,// TODO rename that is key!!!
}
pub struct CachedInfoEI {
  // TODO rename field plus rem option
  pub cached_key : Option<Vec<MultipleErrorInfo>>,
  pub prev_peer : Vec<u8>,// TODO rename that is key!!!
}



pub struct CachedInfoR {
  // TODO rename field plus rem option
  pub cached_key : Option<CachedR>,
  pub prev_peer : Vec<u8>,// TODO rename that is key!!!
}


/// simply vec as regarding algo get are done in push order most of the time
/// second usize is next get index (starting at 0), last is cache id last ix
pub struct CachedInfoManager<P : Peer> (Vec<CachedInfo<P>>, usize, usize,Vec<CachedInfoR>,usize,Vec<CachedInfoE<P>>,usize,Vec<CachedInfoEI>,usize);
impl<P : Peer> TunnelCacheErr<(ErrorWriter,<P as Peer>::Address), MultipleErrorInfo> for CachedInfoManager<P> {
   fn put_errw_tunnel(&mut self, k : &[u8], ssw : (ErrorWriter,<P as Peer>::Address)) -> Result<()> {
    // TODO remove if one...
    self.5.push(CachedInfoE{
      cached_key : Some(ssw),
      prev_peer : k.to_vec(),
    });
    Ok(())
  }

  fn get_errw_tunnel(&mut self, k : &[u8]) -> Result<&mut (ErrorWriter,<P as Peer>::Address)> {
    for i in self.6 .. self.5.len() {
      if self.5[i].prev_peer == k {
        self.6 = i;
        return Ok(self.5[i].cached_key.as_mut().unwrap())
      }
    };
    for i in 0 .. self.6 {
      if self.5[i].prev_peer == k {
        self.6 = i;
        return Ok(self.5[i].cached_key.as_mut().unwrap())
      }
    };
    Err(Error::new(ErrorKind::Other, "Missing content : TODO change trait to return an option in result"))
  }
  fn put_errr_tunnel(&mut self, k : &[u8], v : Vec<MultipleErrorInfo>) -> Result<()> {
    // TODO remove if one...
    self.7.push(CachedInfoEI{
      cached_key : Some(v),
      prev_peer : k.to_vec(),
    });
    Ok(())
  }
 
  fn get_errr_tunnel(&mut self, k : &[u8]) -> Result<&[MultipleErrorInfo]> {
    for i in self.8 .. self.7.len() {
      if self.7[i].prev_peer == k {
        self.8 = i;
        return Ok(self.7[i].cached_key.as_mut().unwrap())
      }
    };
    for i in 0 .. self.8 {
      if self.7[i].prev_peer == k {
        self.8 = i;
        return Ok(self.7[i].cached_key.as_mut().unwrap())
      }
    };
    Err(Error::new(ErrorKind::Other, "Missing content : TODO change trait to return an option in result"))
 
  }
 
}

impl<P : Peer> CacheIdProducer for CachedInfoManager<P> {
  fn new_cache_id (&mut self) -> Vec<u8> {
  let i = self.2;
  self.2 = i + 1; 
  vec!(i as u8, (i + 1) as u8, (i + 2) as u8, (i + 3) as u8)
  }


}

/// TODO type for SSR 
impl<P : Peer> TunnelCache<(CachedW,<P as Peer>::Address),CachedR> for CachedInfoManager<P> {

//  fn put_symw_tunnel(&mut self, k, &[u8], RcRefCell<<CachedW>>) -> Result<()>;
  fn put_symw_tunnel(&mut self, k : &[u8], ssw : (CachedW,<P as Peer>::Address)) -> Result<()> {
    // TODO remove if one...
    self.0.push(CachedInfo{
      cached_key : Some(ssw),
      prev_peer : k.to_vec(),
    });
    Ok(())
  }

  fn get_symw_tunnel(&mut self, k : &[u8]) -> Result<&mut (CachedW,<P as Peer>::Address)> {
    for i in self.1 .. self.0.len() {
      if self.0[i].prev_peer == k {
        self.1 = i;
        return Ok(self.0[i].cached_key.as_mut().unwrap())
      }
    };
    for i in 0 .. self.1 {
      if self.0[i].prev_peer == k {
        self.1 = i;
        return Ok(self.0[i].cached_key.as_mut().unwrap())
      }
    };
    Err(Error::new(ErrorKind::Other, "Missing content : TODO change trait to return an option in result"))
  }

  fn has_symw_tunnel(&mut self, k : &[u8]) -> bool {
    self.get_symw_tunnel(k).is_ok()
  }

  fn put_symr_tunnel(&mut self, r : CachedR) -> Result<Vec<u8>> {
    let k = self.new_cache_id();
    self.3.push(CachedInfoR{
      cached_key : Some(r),
      prev_peer : k.clone(),
    });
    Ok(k)
  }
  fn get_symr_tunnel(&mut self, k : &[u8]) -> Result<&mut CachedR> {
    for i in self.4 .. self.3.len() {
      if self.3[i].prev_peer == k {
        self.4 = i;
        return Ok(self.3[i].cached_key.as_mut().unwrap())
      }
    };
    for i in 0 .. self.4 {
      if self.3[i].prev_peer == k {
        self.4 = i;
        return Ok(self.3[i].cached_key.as_mut().unwrap())
      }
    };

    Err(Error::new(ErrorKind::Other, "Missing content : TODO change trait to return an option in result"))
  }

}

#[derive(Clone)]
pub struct SProv (ShadowTest);
#[derive(Clone)]
pub struct SRead (ShadowTest);
#[derive(Clone)]
pub struct SWrite (ShadowTest);
impl ExtWrite for SWrite {
  #[inline]
  fn write_header<W : Write>(&mut self, w : &mut W) -> Result<()> {
    self.0.write_header(w)
  }
  #[inline]
  fn write_into<W : Write>(&mut self, w : &mut W, cont : &[u8]) -> Result<usize> {
    self.0.write_into(w,cont)
  }
  #[inline]
  fn flush_into<W : Write>(&mut self, w : &mut W) -> Result<()> {
    self.0.flush_into(w)
  }
  #[inline]
  fn write_end<W : Write>(&mut self, w : &mut W) -> Result<()> {
    self.0.write_end(w)
  }
}
impl ExtRead for SRead {
  fn read_header<R : Read>(&mut self, r : &mut R) -> Result<()> {
    self.0.read_header(r)
  }
  #[inline]
  fn read_from<R : Read>(&mut self, r : &mut R, buf : &mut[u8]) -> Result<usize> {
    self.0.read_from(r,buf)
  }
  #[inline]
  fn read_end<R : Read>(&mut self, r : &mut R) -> Result<()> {
    self.0.read_end(r)
  }
}

impl SymProvider<SWrite,SRead> for SProv {
  fn new_sym_key (&mut self) -> Vec<u8> {
    ShadowTest::shadow_simkey()
  }
  // TODO peerkey at 0??
  fn new_sym_writer (&mut self, v : Vec<u8>) -> SWrite {
    let mut st = self.0.clone();
    st.0 = 0;
    st.1 = v[0];
    SWrite(st)
  }
  // TODO peerkey at 0??
  fn new_sym_reader (&mut self, v : Vec<u8>) -> SRead {
    let mut st = self.0.clone();
    st.0 = 0;
    st.1 = v[0];
    SRead(st)
  }
}

#[derive(Clone)]
pub struct TunnelTestConfig<P:Peer> {
    pub me : P,
    pub dest : P,
    pub nbpeer : usize,
    pub route1 : Vec<P>,
    pub route2 : Vec<P>,
    pub input_length : usize,
    pub write_buffer_length : usize,
    pub read_buffer_length : usize,
    pub reply_mode : MultipleReplyMode,
    pub error_mode : MultipleErrorMode,
    pub error_position : usize,
    pub test_mode : TestMode,
}

#[derive(Clone)]
pub enum TunnelMode {
  NoTunnel,
  Tunnel,
  BiTunnel,
  BiTunnelOther,
  NoRepTunnel,
}

/// route provider
pub struct Rp<P : Peer>(bool,Vec<P>, Vec<P>, usize, Vec<P>);

pub struct SingleRp<P : Peer> (Vec<P>);
impl<P : Peer> Rp<P> {
  pub fn new (s : usize,pt : Vec<P>, pt2 : Vec<P>) -> Rp<P> {
    //Rp(false,peer_tests(),peer_tests_2(),s, Vec::new())
    Rp(false,pt,pt2,s, Vec::new())
  }
  pub fn set_size(&mut self, s : usize) {
    self.3 = s;
  }
}

impl<P : Peer> RouteProvider<P> for Rp<P> {
  fn new_route (&mut self, dest : &P) -> Vec<&P> {
    self.0 = !self.0;
    self.4.push(dest.clone());
    let mut r : Vec<&P> = if self.0 {
      self.1[..self.3].iter().collect()
    } else {
      self.2[..self.3].iter().collect()
    };
    r[self.3 - 1] = self.4.last().unwrap();
    r
  }
  /// for bitunnel (arg is still dest our peer address is known to route provider) 
  fn new_reply_route (&mut self, dest : &P) -> Vec<&P> {
    self.0 = !self.0;
    self.4.push(dest.clone());
    let mut r : Vec<&P> = if self.0 {
      self.1[..self.3].iter().rev().collect()
    } else {
      self.2[..self.3].iter().rev().collect()
    };
    r[0] = self.4.last().unwrap();
    r
  }
}
fn new_reply_route_1<P : Clone> (base : &[P], l : usize, dest : &P) -> Vec<P> {

  let mut res : Vec<P> = base[..l].iter().rev().map(|a|a.clone()).collect();
  res[0] = dest.clone();
  res
}


impl<P : Peer> RouteProvider<P> for SingleRp<P> {
  fn new_route (&mut self, dest : &P) -> Vec<&P> {
      self.0.iter().collect()
  }
  fn new_reply_route (&mut self, dest : &P) -> Vec<&P> {
      self.0.iter().rev().collect()
  }
}

/*pub trait ErrorProvider<P : Peer, EI : Info> {
  /// Error infos bases for peers
  fn new_error_route (&mut self, &[&P]) -> Vec<EI>;
}

pub struct ReplyInfoProvider<E : ExtWrite + Clone, TNR : TunnelNoRep,SSW,SSR, SP : SymProvider<SSW,SSR>, RP : RouteProvider<TNR::P>> {
  tunrep : TNR,
  // for different reply route
  symprov : SP,
  routeprov : RP,
  _p : PhantomData<(SSW,SSR)>,
}*/
struct ReplyTraits<P : Peer>(PhantomData<P>);
struct TestTunnelTraits<P : Peer>(PhantomData<P>);
impl<P : Peer> GenTunnelTraits for ReplyTraits<P> {
  type P = P;
  type SSW = SWrite;
  type SSR = SRead;
  type TC = Nope;
  type LW = SizedWindows<TestSizedWindows>;
  type LR = SizedWindows<TestSizedWindows>;
  type RP = Nope;
  type RW = Nope;
  type REP = ReplyInfoProvider<
    P,
    SWrite,
    SRead,
    SProv,
  >;
  type EP = NoErrorProvider;
  type TNR = TunnelNope<P>;
  type EW = ErrorWriter;
}
impl<P : Peer> GenTunnelTraits for TestTunnelTraits<P> {
  type EW = ErrorWriter;
  type P = P;
  type SSW = SWrite;
  type SSR = SRead;
  type TC = CachedInfoManager<P>;
  type LW = SizedWindows<TestSizedWindows>;
  type LR = SizedWindows<TestSizedWindows>;
  type RP = Rp<P>;
  type TNR = Full<ReplyTraits<P>>;
//pub struct ReplyInfoProvider<E : ExtWrite + Clone, TNR : TunnelNoRep,SSW,SSR, SP : SymProvider<SSW,SSR>, RP : RouteProvider<TNR::P>> {
//impl<E : ExtWrite + Clone,P : Peer,TW : TunnelWriter, TNR : TunnelNoRep<P=P,TW=TW>,SSW,SSR,SP : SymProvider<SSW,SSR>,RP : RouteProvider<P>> ReplyProvider<P, ReplyInfo<E,P,TW>,SSW,SSR> for ReplyInfoProvider<E,TNR,SSW,SSR,SP,RP> {
//
//impl<E : ExtWrite, P : Peer, RI : RepInfo, EI : Info> TunnelWriter for FullW<RI,EI,P,E> {
//type TW = FullW<ReplyInfo<TT::LW,TT::P,TT::RW>, MultiErrorInfo<TT::LW,TT::RW>, TT::P, TT::LW>;
  type RW = FullW<MultipleReplyInfo<Self::P>, MultipleErrorInfo,Self::P, Self::LW,Nope>;
  //type RW = TunnelWriterFull<FullW<MultipleReplyInfo<Self::LW,Self::P,Nope>, MultiErrorInfo<Self::LW,Nope>,Self::P, Self::LW>>;
  type REP = ReplyInfoProvider<
//    SizedWindows<TestSizedWindows>,
//    Full<ReplyTraits<P>>,
    P,
    SWrite,
    SRead,
    SProv,
  >;
  type EP = MulErrorProvider;
}
#[derive(Clone)]
pub enum TestMode {
  NoReply,
  Reply(usize), // param nb reply
  // ErrorInQuery
}
fn new_full_tunnel<P : Peer> (tc : &TunnelTestConfig<P>, from : &P, ixcache : usize) 
                          -> Full<TestTunnelTraits<P>>
where <<P as Peer>::Shadow as Shadow>::ShadowMode : Eq
{
  let mut route_prov = Rp::new (tc.nbpeer,tc.route1.clone(),tc.route2.clone());
  let mut cache : CachedInfoManager<P> = CachedInfoManager(Vec::new(),0,ixcache,Vec::new(),0,Vec::new(),0,Vec::new(),0);
  let TunnelTestConfig {
    me : _,
    dest : dest,
    nbpeer : nbpeer,
    route1 : route1,
    route2 : route2,
    input_length : input_length,
    write_buffer_length : write_buffer_length,
    read_buffer_length : read_buffer_length,
    reply_mode : reply_mode,
    error_mode : error_mode,
    error_position : error_position,
    test_mode : test_mode,
  } = tc.clone();

  let route_rep : Vec<P> = match reply_mode {
    MultipleReplyMode::OtherRoute => new_reply_route_1 (&route2[..],nbpeer, &dest),
    _ => new_reply_route_1(&route1[..],nbpeer, &dest),
  };
  // TODO error in reply ?? TODO specific full for it given reply full!!(no route, ...) : must be
  // use with getReplyWriter
  let tunnel_reply : Full<ReplyTraits<P>> = Full {
    me : dest.clone(),
    // reply mode here is major
    reply_mode : MultipleReplyMode::RouteReply,
    error_mode : MultipleErrorMode::NoHandling,
    cache : Nope,
    //  pub sym_prov : TT::SP,
    route_prov : Nope,
    reply_prov : ReplyInfoProvider {
      mode : MultipleReplyMode::RouteReply,
      symprov : SProv(ShadowTest(0,0,ShadowModeTest::SimpleShift)),
      _p : PhantomData,
    },
    error_prov : NoErrorProvider,
    rng : thread_rng(),
    limiter_proto_w : SizedWindows::new(TestSizedWindows),
    limiter_proto_r : SizedWindows::new(TestSizedWindows),
    tunrep : TunnelNope::new(),
    reply_once_buf_size : 256,
    _p : PhantomData,
  };

  let rip = ReplyInfoProvider {
    mode : reply_mode.clone(),
    symprov : SProv(ShadowTest(0,0,ShadowModeTest::SimpleShift)),
    _p : PhantomData,
  };

  Full {
    me : from.clone(),
    reply_mode : reply_mode.clone(),
    error_mode : error_mode.clone(),
    cache : cache,
    //  pub sym_prov : TT::SP,
    route_prov : route_prov,
    reply_prov : rip,
    error_prov : MulErrorProvider::new(error_mode.clone()).unwrap(),
    rng : thread_rng(),
    limiter_proto_w : SizedWindows::new(TestSizedWindows),
    limiter_proto_r : SizedWindows::new(TestSizedWindows),
    tunrep : tunnel_reply,
    reply_once_buf_size : 256,
    _p : PhantomData,
  }
}
                        
 
/// main tunnel test : send message over a route
pub fn tunnel_test<P : Peer> (  tc : TunnelTestConfig<P>)
where <<P as Peer>::Shadow as Shadow>::ShadowMode : Eq
{
  let mut ixcache = 0;
 // TODO from here generic function
  let mut tunnels : Vec<_> = tc.route1[..tc.nbpeer].iter().map(|p|{
    ixcache = ixcache + 1;
    new_full_tunnel(&tc, &p,ixcache)
  }).collect();

  let (tunn_we,dest) = tunnels[0].new_writer(&tc.dest);
  assert!(dest == tunnels[1].me.to_address());
  let output : Cursor<Vec<u8>> = Cursor::new(Vec::new());
  send_test(tc, tunn_we, tunnels, output);
}

fn reply_test<P : Peer> (mut tc : TunnelTestConfig<P>, mut dr : 
                         DestFull<FullR<MultipleReplyInfo<P>,MultipleErrorInfo,P,SizedWindows<TestSizedWindows>>,SRead, SizedWindows<TestSizedWindows>>
                         , mut input : Cursor<Vec<u8>>, tunnel : &mut Full<TestTunnelTraits<P>>)
where <<P as Peer>::Shadow as Shadow>::ShadowMode : Eq
{
   let mut ixcache = 127;//half of 8 bit pool
   let reply_mode = tc.reply_mode.clone();
   let route_rep : Vec<P> = match reply_mode {
     MultipleReplyMode::OtherRoute => new_reply_route_1 (&tc.route2[..],tc.nbpeer, &tc.dest),
     _ => new_reply_route_1(&tc.route1[..],tc.nbpeer, &tc.dest),
   };

   let mut output : Cursor<Vec<u8>> = Cursor::new(Vec::new());

   let (mut rw, dest) = tunnel.new_reply_writer(&mut dr, &mut input).unwrap();
   let tunnelsrep : Vec<_> = route_rep.iter().map(|p|{
     ixcache += 1;
    new_full_tunnel(&tc, &p, ixcache)
   }).collect();
   assert!(dest == tunnelsrep[1].me.to_address());
   tunnel.reply_writer_init(&mut rw, &mut dr, &mut input, &mut output).unwrap();



   // write 
   send_test(tc, rw, tunnelsrep, output);
}
fn reply_cached_test<P : Peer> (mut tc : TunnelTestConfig<P>, mut dr : 
                         DestFull<FullR<MultipleReplyInfo<P>,MultipleErrorInfo,P,SizedWindows<TestSizedWindows>>,SRead, SizedWindows<TestSizedWindows>>
                         , mut input : Cursor<Vec<u8>>, mut tunnels : Vec<Full<TestTunnelTraits<P>>>)
where <<P as Peer>::Shadow as Shadow>::ShadowMode : Eq
{
   let reply_mode = tc.reply_mode.clone();

   let mut output : Cursor<Vec<u8>> = Cursor::new(Vec::new());

   tunnels.reverse();
   let (mut rw, dest) = tunnels[0].new_reply_writer(&mut dr, &mut input).unwrap();
   assert!(dest == tunnels[1].me.to_address());
   tunnels[0].reply_writer_init(&mut rw, &mut dr, &mut input, &mut output).unwrap();


   // write 
   send_test(tc, rw, tunnels, output);
}

fn send_test<P : Peer, W : ExtWrite> (mut tc : TunnelTestConfig<P>, mut tunn_we : W, 
                         mut tunnels : Vec<Full<TestTunnelTraits<P>>>, mut output : Cursor<Vec<u8>>)
where <<P as Peer>::Shadow as Shadow>::ShadowMode : Eq
{
 let TunnelTestConfig {
     me : me,
     dest : dest,
     nbpeer : nbpeer,
     route1 : route1,
     route2 : route2,
     input_length : input_length,
     write_buffer_length : write_buffer_length,
     read_buffer_length : read_buffer_length,
     reply_mode : reply_mode,
     error_mode : error_mode,
     error_position : error_position,
     test_mode : test_mode,
 } = tc.clone();


  let mut inputb = vec![0;input_length];
  let mut rnd = OsRng::new().unwrap();
  rnd.fill_bytes(&mut inputb);
  let input = inputb;
/*  let vec_route : Vec<(usize,&P)> = route.iter().map(|p|{
    let errorid = rnd.gen();
    (errorid,p)
  }).collect();*/
  // send message test
/*  let ocr = {
    let mut tunn_we = TunnelWriterExt::new(
      &vec_route[..],
      SizedWindows::new(TestSizedWindows),
      tmode.clone(),
      TunnelState::QueryOnce,// query once default
      None,
      shead.clone(),
      scont.clone(),
      None,// no error routes
      None,// no specific reply route
      cache_ids[0].clone(),
      Some(SizedWindows::new(TestSizedWindows)),// for cached reader
    ).unwrap();
*/
    { // s lifetime to write end on release from compw
      let mut tunn_w = CompW::new(&mut output, &mut tunn_we);
      
      // tunn_w.write_all(&input[..input_length]).unwrap();
      let mut ix = 0;
      while ix < input_length {
        if ix + write_buffer_length < input_length {
          ix += tunn_w.write(&input[ix..ix + write_buffer_length]).unwrap();
        } else {
          ix += tunn_w.write(&input[ix..]).unwrap();
        }
      }
      tunn_w.flush().unwrap();
    }


    // tunnel to use
    let mut emptybuf = [];
    let mut ix;
    let mut readbuf = vec![0;read_buffer_length];
 
    let mut tunn_re;
    // TODO proxy stuff
   
    let nbtoprox = if nbpeer > 2 {nbpeer - 2} else {0};
    let mut nbp = 0;
    for i in 1 .. nbpeer - 1 {
      let from_add = tunnels[i - 1].me.to_address();
      let to_add = tunnels[i + 1].me.to_address();

      let mut input_v = Cursor::new(output.into_inner());
      output = Cursor::new(Vec::new());
      let (mut proxy, dest) =  { 
        let tunnel = &mut tunnels[i];
        tunn_re = tunnel.new_reader(&from_add);
        assert!(tunn_re.is_dest() == None);
        tunn_re.read_header(&mut input_v).unwrap();
        if tunn_re.is_dest() == None {
          tunnel.init_dest(&mut tunn_re).unwrap();
        }
        assert!(tunn_re.is_dest() == Some(false));
        assert!(tunn_re.is_err() == Some(false));
        tunnel.new_proxy_writer(tunn_re).unwrap()
      };
      assert!(dest == to_add);
      proxy.read_header(&mut input_v).unwrap();

      proxy.write_header(&mut output).unwrap();
/*if let TestMode::Reply(0) = test_mode {
  panic!("{:?}", output.into_inner());
}*/
 
      while  {
        let l = proxy.read_from(&mut input_v, &mut readbuf).unwrap();
        ix = 0;
        while ix < l {
          ix += proxy.write_into(&mut output, &mut readbuf[..l]).unwrap();
        }


        l > 0 // unknown length
      } {}
      proxy.read_end(&mut input_v).unwrap();
      proxy.write_end(&mut output).unwrap();
      if i == error_position {
        let nbpeer = error_position + 1;
        let mut tunn_err = &mut tunnels[0..error_position + 1];
        tunn_err.reverse();
        let (mut te, dest) = tunn_err[0].new_error_writer(&mut proxy.get_reader()).unwrap();
        assert!(dest == tunn_err[1].me.to_address());
        send_error (nbpeer, te, &mut tunn_err);
        return;
      }

      nbp += 1;
    }
    // not for dest peer TODO add dest test : after read dest
      // reply error
/*if let TestMode::Reply(0) = test_mode {
  panic!("{:?}", output.into_inner());
}*/
    assert!(nbtoprox == nbp);
    
    // dest stuff

    let mut dest_reader;
    let mut input_v;
    {
      let from_add = tunnels[nbpeer - 2].me.to_address();
      let tunnel = &mut tunnels[nbpeer - 1];
      tunn_re = tunnel.new_reader(&from_add);
      input_v = Cursor::new(output.into_inner());
      tunn_re.read_header(&mut input_v).unwrap();
      if tunn_re.is_dest() == None {
        tunnel.init_dest(&mut tunn_re).unwrap();
      }
      assert!(tunn_re.is_dest() == Some(true));
      assert!(tunn_re.is_err() == Some(false));

      dest_reader = tunnel.new_dest_reader(tunn_re, &mut input_v).unwrap();
        /* issue with CompR dropping abnormally : commenting for now
        let mut tunn_r = CompR::new(&mut input_v, &mut dest_reader);
        assert_eq!(0,tunn_r.read(&mut emptybuf[..]).unwrap());
        panic!("y");
        ix = 0;
        while ix < input_length { // known length
          let l = if ix + readbuf.len() < input.len() { 
            tunn_r.read( &mut readbuf).unwrap()
          } else {
            tunn_r.read( &mut readbuf[..input.len() - ix]).unwrap()
          };

          assert!(l!=0);

          assert_eq!(&readbuf[..l], &input[ix..ix + l]);
          ix += l;
        }*/
        //let mut buf3 = vec![0;1024];   let a3 = input_v.read( &mut buf3[..]).unwrap(); panic!("b : {:?}", &buf3[..a3]);
        dest_reader.read_header(&mut input_v).unwrap();
        assert_eq!(0,dest_reader.read_from(&mut input_v, &mut emptybuf[..]).unwrap());
        ix = 0;

        let mut it = 0;
        while ix < input_length { // known length
          let l = if ix + readbuf.len() < input.len() { 
            dest_reader.read_from(&mut input_v, &mut readbuf).unwrap()
          } else {
            dest_reader.read_from(&mut input_v, &mut readbuf[..input.len() - ix]).unwrap()
          };

          assert!(l!=0);

          assert_eq!(&readbuf[..l], &input[ix..ix + l],"on it {:?}",it);
          it += 1;
          ix += l;
        }
      }
      match test_mode {
        TestMode::NoReply | TestMode::Reply(0) => {
          dest_reader.read_to_end(&mut input_v,&mut readbuf).unwrap();
          let l = input_v.read(&mut readbuf).unwrap();
          assert_eq!(l,0);
        },
        TestMode::Reply(nbr) => {
          tc.test_mode = TestMode::Reply(nbr - 1);
          match reply_mode {
            MultipleReplyMode::Route => reply_test(tc, dest_reader, input_v, &mut tunnels[nbpeer - 1]),
            MultipleReplyMode::OtherRoute => reply_test(tc, dest_reader, input_v, &mut tunnels[nbpeer - 1]),
            MultipleReplyMode::NoHandling => (),
            MultipleReplyMode::CachedRoute => reply_cached_test(tc, dest_reader, input_v, tunnels),
            _ => panic!("Test case not covered"),
          }
        },
      }
        

}
fn send_error<P : Peer, EW : TunnelErrorWriter> (nbpeer : usize, mut tunn_e : EW, 
                         mut tunnels : &mut [Full<TestTunnelTraits<P>>])
where <<P as Peer>::Shadow as Shadow>::ShadowMode : Eq
{

 let mut output = Cursor::new(Vec::new());
 tunn_e.write_error(&mut output).unwrap();
   
 let mut tunn_re;
 let mut input_v;
    let nbtoprox = if nbpeer > 2 {nbpeer - 2} else {0};
    let mut nbp = 0;
    for i in 1 .. nbpeer - 1 {
      let from_add = tunnels[i - 1].me.to_address();
      let to_add = tunnels[i + 1].me.to_address();
      let tunnel = &mut tunnels[i];
      tunn_re = tunnel.new_reader(&from_add);
      assert!(tunn_re.is_dest() == None);
      assert!(tunn_re.is_err() == None);
      input_v = Cursor::new(output.into_inner());
      output = Cursor::new(Vec::new());
      tunn_re.read_header(&mut input_v).unwrap();
      if tunn_re.is_dest() == None {
        tunnel.init_dest(&mut tunn_re).unwrap();
      }
      assert!(tunn_re.is_dest() == Some(false));
      assert!(tunn_re.is_err() == Some(true));
      let (mut proxy, dest) = tunnel.proxy_error_writer(&mut tunn_re).unwrap();
      assert!(dest == to_add);
      proxy.write_error(&mut output).unwrap();
      nbp += 1;
    }

    assert!(nbtoprox == nbp);
      let from_add = tunnels[nbpeer - 2].me.to_address();
      let tunnel = &mut tunnels[nbpeer - 1];
      tunn_re = tunnel.new_reader(&from_add);
      input_v = Cursor::new(output.into_inner());
      tunn_re.read_header(&mut input_v).unwrap();
      if tunn_re.is_dest() == None {
        tunnel.init_dest(&mut tunn_re).unwrap();
      }
      assert!(tunn_re.is_dest() == Some(true));
      assert!(tunn_re.is_err() == Some(true));

      let err_ix = tunnel.read_error(&mut tunn_re).unwrap();
      assert!(err_ix != 0);
      assert!(err_ix == nbpeer - 1);

}
 

fn peer_tests () -> Vec<PeerTest> {
[ PeerTest {
    nodeid: "toid1".to_string(),
    address : LocalAdd(1),
    keyshift: 2,
    modesh : test_peer_mode(),
},
 PeerTest  {
    nodeid: "toid2".to_string(),
    address : LocalAdd(2),
    keyshift: 3,
    modesh : test_peer_mode(),
},
 PeerTest {
    nodeid: "toid3".to_string(),
    address : LocalAdd(3),
    keyshift: 4,
    modesh : test_peer_mode(),
},
 PeerTest {
    nodeid: "toid4".to_string(),
    address : LocalAdd(4),
    keyshift: 5,
    modesh : test_peer_mode(),
},
 PeerTest {
    nodeid: "toid5".to_string(),
    address : LocalAdd(5),
    keyshift: 6,
    modesh : test_peer_mode(),
},
 PeerTest {
    nodeid: "toid6".to_string(),
    address : LocalAdd(6),
    keyshift: 5,
    modesh : test_peer_mode(),
},
].to_vec()
}

fn peer_tests_2 () -> Vec<PeerTest> {
[ PeerTest {
    nodeid: "toid1".to_string(),
    address : LocalAdd(1),
    keyshift: 2,
    modesh : test_peer_mode(),
},
PeerTest {
    nodeid: "toid7".to_string(),
    address : LocalAdd(7),
    keyshift: 9,
    modesh : test_peer_mode(),
},
 PeerTest  {
    nodeid: "toid8".to_string(),
    address : LocalAdd(8),
    keyshift: 11,
    modesh : test_peer_mode(),
},
 PeerTest {
    nodeid: "toid9".to_string(),
    address : LocalAdd(9),
    keyshift: 6,
    modesh : test_peer_mode(),
},
 PeerTest {
    nodeid: "toid10".to_string(),
    address : LocalAdd(10),
    keyshift: 2,
    modesh : test_peer_mode(),
},
 PeerTest {
    nodeid: "toid11".to_string(),
    address : LocalAdd(11),
    keyshift: 1,
    modesh : test_peer_mode(),
},
].to_vec()
}

#[inline]
fn test_peer_mode() -> ShadowModeTest {
  ShadowModeTest::SimpleShift
//  ShadowModeTest::SimpleShiftNoHead
//  ShadowModeTest::NoShadow
}

#[test]
fn tunnel_nohop_noreptunnel_1() {
  tunnel_testpeer_test(2, MultipleReplyMode::Route, MultipleErrorMode::NoHandling, 500, 360, 130, TestMode::NoReply);
}

#[test]
fn tunnel_nohop_noreptunnel_1_c() {
  tunnel_testpeer_test(2, MultipleReplyMode::CachedRoute, MultipleErrorMode::NoHandling, 500, 360, 130, TestMode::NoReply);
}
#[test]
fn tunnel_nohop_reptunnel_1_c() {
  tunnel_testpeer_test(2, MultipleReplyMode::CachedRoute, MultipleErrorMode::NoHandling, 500, 360, 130, TestMode::Reply(1));
}
#[test]
fn tunnel_nohop_reptunnel_2_c() {
  tunnel_testpeer_test(3, MultipleReplyMode::CachedRoute, MultipleErrorMode::NoHandling, 500, 360, 130, TestMode::Reply(1));
}

#[test]
fn tunnel_nohop_reptunnel_2_c_err() {
  tunnel_testpeer_test_err(3, MultipleReplyMode::CachedRoute, MultipleErrorMode::CachedRoute, 500, 360, 130, TestMode::Reply(1),1);
}

#[test]
fn tunnel_nohop_reptunnel_3_c() {
  tunnel_testpeer_test(6, MultipleReplyMode::CachedRoute, MultipleErrorMode::NoHandling, 500, 360, 130, TestMode::Reply(1));
}
#[test]
fn tunnel_nohop_reptunnel_3_c_err() {
  tunnel_testpeer_test_err(6, MultipleReplyMode::CachedRoute, MultipleErrorMode::CachedRoute, 500, 360, 130, TestMode::Reply(1),4);
}

#[test]
fn tunnel_nohop_reptunnel_3_c_ne() {
  tunnel_testpeer_test(6, MultipleReplyMode::CachedRoute, MultipleErrorMode::CachedRoute, 500, 360, 130, TestMode::Reply(1));
}


#[test]
fn tunnel_nohop_noreptunnel_3_c() {
  tunnel_testpeer_test(6, MultipleReplyMode::CachedRoute, MultipleErrorMode::NoHandling, 500, 360, 130, TestMode::NoReply);
}


#[test]
fn tunnel_nohop_reptunnel_1() {
  tunnel_testpeer_test(2, MultipleReplyMode::Route, MultipleErrorMode::NoHandling, 500, 360, 130, TestMode::Reply(1));
}
#[test]
fn tunnel_nohop_reptunnel_2() {
  tunnel_testpeer_test(3, MultipleReplyMode::OtherRoute, MultipleErrorMode::NoHandling, 500, 360, 130, TestMode::Reply(1));
}
#[test]
fn tunnel_nohop_reptunnel_3() {
  tunnel_testpeer_test(6, MultipleReplyMode::OtherRoute, MultipleErrorMode::NoHandling, 500, 360, 130, TestMode::Reply(1));
}

#[test]
fn tunnel_nohop_reptunnel_3_bis() {
  tunnel_testpeer_test(6, MultipleReplyMode::Route, MultipleErrorMode::NoHandling, 500, 360, 130, TestMode::Reply(1));
}

#[test]
fn tunnel_nohop_noreptunnel_2() {
  tunnel_testpeer_test(3, MultipleReplyMode::OtherRoute, MultipleErrorMode::NoHandling, 500, 360, 130, TestMode::NoReply);
}

#[test]
fn tunnel_nohop_noreptunnel_3() {
  tunnel_testpeer_test(6, MultipleReplyMode::NoHandling, MultipleErrorMode::NoHandling, 500, 360, 130, TestMode::NoReply);
}


pub fn tunnel_testpeer_test(nbpeer : usize, replymode : MultipleReplyMode, errormode : MultipleErrorMode, input_length : usize, write_buffer_length : usize, read_buffer_length : usize, test_mode : TestMode) {
  tunnel_testpeer_test_err(nbpeer, replymode, errormode, input_length, write_buffer_length, read_buffer_length, test_mode, 0);
}
pub fn tunnel_testpeer_test_err(nbpeer : usize, replymode : MultipleReplyMode, errormode : MultipleErrorMode,  input_length : usize, write_buffer_length : usize, read_buffer_length : usize, test_mode : TestMode, err_position : usize) {
  let r = peer_tests();
  let tc = TunnelTestConfig {
    me : r[0].clone(),
    dest : r[nbpeer - 1].clone(),
    nbpeer : nbpeer,
    route1 : peer_tests(),
    route2 : peer_tests_2(),
    input_length : input_length,
    write_buffer_length : write_buffer_length,
    read_buffer_length : read_buffer_length,
    reply_mode : replymode,
    error_mode : errormode,
    error_position : err_position,
    test_mode : test_mode,
  };
  tunnel_test(tc); 
}

