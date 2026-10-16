(async () => {
  const read = fn => { try { return fn(); } catch (e) { return {error:e.name}; } };
  const hash = s => { let h=2166136261; for(let i=0;i<s.length;i++) h=Math.imul(h^s.charCodeAt(i),16777619); return (h>>>0).toString(16); };
  const canvas=document.createElement('canvas'); canvas.width=300; canvas.height=80;
  const ctx=canvas.getContext('2d'); ctx.fillStyle='#f60'; ctx.fillRect(10,10,90,30);
  ctx.font='18px Arial'; ctx.fillStyle='#069'; ctx.fillText('Xaga comparison Ω 😀',4,42);
  const gl=document.createElement('canvas').getContext('webgl');
  const debug=gl && gl.getExtension('WEBGL_debug_renderer_info');
  const result={
    userAgent:navigator.userAgent, appVersion:navigator.appVersion, platform:navigator.platform,
    vendor:navigator.vendor, language:navigator.language, languages:navigator.languages,
    hardwareConcurrency:navigator.hardwareConcurrency, deviceMemory:navigator.deviceMemory ?? null,
    maxTouchPoints:navigator.maxTouchPoints, cookieEnabled:navigator.cookieEnabled,
    doNotTrack:navigator.doNotTrack ?? null, webdriver:navigator.webdriver,
    timezone:read(()=>Intl.DateTimeFormat().resolvedOptions().timeZone), timezoneOffset:new Date().getTimezoneOffset(),
    screen:{width:screen.width,height:screen.height,availWidth:screen.availWidth,availHeight:screen.availHeight,colorDepth:screen.colorDepth,pixelDepth:screen.pixelDepth},
    window:{innerWidth,innerHeight,outerWidth,outerHeight,devicePixelRatio},
    secureContext:isSecureContext,
    media:Object.fromEntries(['prefers-color-scheme: dark','prefers-reduced-motion: reduce','color-gamut: p3','pointer: fine','hover: hover'].map(q=>[q,matchMedia('('+q+')').matches])),
    apis:Object.fromEntries(['RTCPeerConnection','AudioContext','OfflineAudioContext','WebGLRenderingContext','OffscreenCanvas','SharedArrayBuffer'].map(k=>[k,typeof window[k]])),
    peerConnection:read(()=>{ const p=new RTCPeerConnection(); p.close(); return 'constructible'; }),
    canvasHash:read(()=>hash(canvas.toDataURL())),
    fontWidths:read(()=>Object.fromEntries(['Arial','Helvetica','Times New Roman','Courier New','Menlo','Monaco','system-ui'].map(f=>{ctx.font='16px "'+f+'"';return [f,ctx.measureText('mmmmmmmmmmlliΩ').width];}))),
    webgl:gl?{vendor:gl.getParameter(gl.VENDOR),renderer:gl.getParameter(gl.RENDERER),unmaskedVendor:debug?gl.getParameter(debug.UNMASKED_VENDOR_WEBGL):null,unmaskedRenderer:debug?gl.getParameter(debug.UNMASKED_RENDERER_WEBGL):null,maxTextureSize:gl.getParameter(gl.MAX_TEXTURE_SIZE),extensions:gl.getSupportedExtensions()}:null,
    plugins:Array.from(navigator.plugins,p=>p.name),
    loaderGlobals:['__LOADER_ID__','__LOADER_LABEL__','__LOADER_URL__','__xagaCaptureImage','__TAURI_INTERNALS__'].filter(k=>k in window)
  };
  try {
    const a=new OfflineAudioContext(1,4096,44100), o=a.createOscillator(), c=a.createDynamicsCompressor();
    o.type='triangle';o.frequency.value=10000;o.connect(c);c.connect(a.destination);o.start();
    const buffer=await a.startRendering(); result.audioHash=hash(Array.from(buffer.getChannelData(0)).join(','));
  } catch(e) {result.audioHash={error:e.name};}
  await fetch('/result'+location.search,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(result)});
  document.body.textContent='Fingerprint comparison complete. Results saved locally; you can close this tab.';
})();
