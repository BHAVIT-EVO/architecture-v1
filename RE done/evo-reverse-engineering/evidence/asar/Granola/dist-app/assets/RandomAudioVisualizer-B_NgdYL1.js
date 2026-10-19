try{let e=typeof window<`u`?window:typeof global<`u`?global:typeof globalThis<`u`?globalThis:typeof self<`u`?self:{},t=new e.Error().stack;t&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[t]=`a748e10b-476d-4097-9993-57b03217a432`,e._sentryDebugIdIdentifier=`sentry-dbid-a748e10b-476d-4097-9993-57b03217a432`)}catch{}import"./_sentry-release-injection-file-DZROVGkF.js";import{a as e}from"./rolldown-runtime-CMh1wmV-.js";import{t}from"./react-DLCF0spK.js";import{Pa as n,vt as r}from"./feature-flags.generated-DHGzSRUt.js";import{t as i}from"./compiler-runtime-DK4A0O2g.js";import{t as a}from"./jsx-runtime-DwLIIa17.js";import{t as o}from"./utils-4opeJDfN.js";import{t as s}from"./BaseIcon-Ddq_fQrn.js";import{Nt as c}from"./store-DCBs6dIR.js";import"./cn-DOCCIXLV.js";import{t as l}from"./useFeatureFlag-ZdA6ABHX.js";function u(e,t={}){let n=t.doc??document,r=null,i=()=>{!r&&!n.hidden&&(r=e())},a=()=>{r&&=(r(),null)};i();let o=()=>{n.hidden?a():i()};return n.addEventListener(`visibilitychange`,o),()=>{n.removeEventListener(`visibilitychange`,o),a()}}var d=i(),f=e(t(),1),p=a(),m=(0,f.forwardRef)((e,t)=>{let n=(0,d.c)(5),r,i;n[0]===Symbol.for(`react.memo_cache_sentinel`)?(r=(0,p.jsx)(`circle`,{stroke:`currentColor`,strokeWidth:`var(--icon-stroke-width)`,strokeLinecap:`round`,strokeLinejoin:`round`,cx:`12`,cy:`12`,r:`9.25`}),i=(0,p.jsx)(`path`,{d:`M7.75 12.9231L10.5625 15.75L15.25 8.75`,stroke:`currentColor`,strokeWidth:`var(--icon-stroke-width)`,strokeLinecap:`round`,strokeLinejoin:`round`}),n[0]=r,n[1]=i):(r=n[0],i=n[1]);let a;return n[2]!==e||n[3]!==t?(a=(0,p.jsxs)(s,{ref:t,...e,children:[r,i]}),n[2]=e,n[3]=t,n[4]=a):a=n[4],a});m.displayName=`IconCheckCircle2`;var h=(0,f.forwardRef)((e,t)=>{let n=(0,d.c)(6),r,i,a;n[0]===Symbol.for(`react.memo_cache_sentinel`)?(r=(0,p.jsx)(`path`,{d:`M2.00097 8.95914C2.00097 8.11937 2.74718 7.43859 3.66765 7.43859C4.58813 7.43859 5.33432 8.11937 5.33432 8.95914V15.0414C5.33432 15.8811 4.58813 16.56195 3.66765 16.56195C2.74718 16.56195 2.00097 15.8811 2.00097 15.0414V8.95914Z`,fill:`currentColor`}),i=(0,p.jsx)(`path`,{d:`M18.66765 8.95914C18.66765 8.11937 19.4139 7.43859 20.3343 7.43859C21.25485 7.43859 22.00095 8.11937 22.00095 8.95914V15.0414C22.00095 15.8811 21.25485 16.56195 20.3343 16.56195C19.4139 16.56195 18.66765 15.8811 18.66765 15.0414V8.95914Z`,fill:`currentColor`}),a=(0,p.jsx)(`path`,{d:`M10.33431 4.39751C10.33431 3.55773 11.0805 2.87696 12.00098 2.87696C12.92145 2.87696 13.66764 3.55773 13.66764 4.39751V19.60305C13.66764 20.44275 12.92145 21.1236 12.00098 21.1236C11.0805 21.1236 10.33431 20.44275 10.33431 19.60305V4.39751Z`,fill:`currentColor`}),n[0]=r,n[1]=i,n[2]=a):(r=n[0],i=n[1],a=n[2]);let o;return n[3]!==e||n[4]!==t?(o=(0,p.jsxs)(s,{ref:t,...e,children:[r,i,a]}),n[3]=e,n[4]=t,n[5]=o):o=n[5],o});h.displayName=`IconDancingBars`;var g=.27,_=200/32767;function v(){let e=_;return t=>(e=Math.max(_,e*g,t),t/e)}function y(e){let t=v(),n=v();return window.electron.onAudioVolume(r=>{e({system:t(r.system),microphone:n(r.microphone)})})}var b=`h-[16px] w-[3px] rounded-full transition-transform duration-300 ease-in-out bg-[var(--barcolor)]`,x=`flex h-[10px] w-[13px] justify-between gap-[2px] overflow-hidden`,S=60,C=1e3/S,w=1/S,T=`
precision highp float;
attribute vec2 aPosition;
varying vec2 vPosition;
varying vec3 vY;
varying vec3 vPreviousY;
uniform float uTime;
uniform vec3 uFrom;
uniform vec3 uTo;
uniform vec3 uStart;
uniform vec3 uDuration;

float ease(float x) {
  float lo = 0.0;
  float hi = 1.0;
  float t = 0.0;
  for (int i = 0; i < 16; i++) {
    t = (lo + hi) * 0.5;
    float u = 1.0 - t;
    float bx = 1.2 * u * u * t + 0.6 * u * t * t + t * t * t;
    if (bx < x) lo = t; else hi = t;
  }
  return 3.0 * (1.0 - t) * t * t + t * t * t;
}

float position(float from, float to, float start, float duration, float time) {
  float progress = clamp((time - start) / max(duration, 0.00001), 0.0, 1.0);
  return mix(from, to, ease(progress));
}

vec3 positions(float time) {
  return vec3(
    position(uFrom.x, uTo.x, uStart.x, uDuration.x, time),
    position(uFrom.y, uTo.y, uStart.y, uDuration.y, time),
    position(uFrom.z, uTo.z, uStart.z, uDuration.z, time));
}

void main() {
  vPosition = (aPosition + 1.0) * 0.5;
  vY = positions(uTime);
  vPreviousY = positions(uTime - ${w});
  gl_Position = vec4(aPosition, 0.0, 1.0);
}`,E=`
precision highp float;
varying vec2 vPosition;
varying vec3 vY;
varying vec3 vPreviousY;
uniform vec2 uSize;
uniform float uPixel;
uniform vec3 uX;
uniform vec2 uBarSize;
uniform vec4 uColor;

float capsule(vec2 p, float x, float top, float height) {
  float radius = uBarSize.x * 0.5;
  vec2 size = vec2(uBarSize.x, height);
  vec2 center = vec2(x + radius, top + height * 0.5);
  vec2 q = abs(p - center) - (size * 0.5 - radius);
  float distance = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
  return 1.0 - smoothstep(-uPixel * 0.5, uPixel * 0.5, distance);
}

float bar(vec2 p, float x, float y, float previousY) {
  float lo = min(y, previousY);
  float hi = max(y, previousY);
  float travel = hi - lo;
  // A pixel the cap swept past was covered for only part of the exposure.
  float exposure = travel > 0.001 ? clamp((p.y - lo) / travel, 0.0, 1.0) : 1.0;
  float trail = capsule(p, x, lo, uBarSize.y + travel) * exposure;
  return max(capsule(p, x, hi, uBarSize.y), trail);
}

void main() {
  vec2 p = vPosition * uSize;
  p.y = min(p.y, uSize.y - p.y);
  float coverage = max(max(
    bar(p, uX.x, vY.x, vPreviousY.x),
    bar(p, uX.y, vY.y, vPreviousY.y)),
    bar(p, uX.z, vY.z, vPreviousY.z));
  float alpha = coverage * uColor.a;
  gl_FragColor = vec4(uColor.rgb * alpha, alpha);
}`;function D(e){if(e<=0)return 0;if(e>=1)return 1;let t=0,n=1,r=0;for(let i=0;i<16;i++){r=(t+n)/2;let i=1-r;1.2*i*i*r+.6*i*r*r+r*r*r<e?t=r:n=r}return 3*(1-r)*r*r+r*r*r}function O(e,t,n){let r=e.getContext(`webgl`,{alpha:!0,premultipliedAlpha:!0,antialias:!1,depth:!1,stencil:!1,powerPreference:`low-power`,failIfMajorPerformanceCaveat:!0});if(!r)return;let i=[],a=r.createProgram(),s=r.createBuffer(),c=(t=!e.isConnected)=>{r.deleteBuffer(s),r.deleteProgram(a),i.forEach(e=>r.deleteShader(e)),t&&r.getExtension(`WEBGL_lose_context`)?.loseContext()},l=(e,t)=>{let n=r.createShader(e);if(!n)throw Error(`Unable to allocate audio visualizer shader`);if(i.push(n),r.shaderSource(n,t),r.compileShader(n),!r.getShaderParameter(n,r.COMPILE_STATUS))throw Error(r.getShaderInfoLog(n)??`Shader compilation failed`);return n};try{if(!a||!s)throw Error(`Unable to allocate WebGL resources`);if(r.attachShader(a,l(r.VERTEX_SHADER,T)),r.attachShader(a,l(r.FRAGMENT_SHADER,E)),r.linkProgram(a),!r.getProgramParameter(a,r.LINK_STATUS))throw Error(r.getProgramInfoLog(a)??`Shader linking failed`)}catch(e){console.warn(`Audio visualizer WebGL unavailable; using DOM bars`,e),c(!0);return}if(!a)return;r.useProgram(a),r.bindBuffer(r.ARRAY_BUFFER,s),r.bufferData(r.ARRAY_BUFFER,new Float32Array([-1,-1,3,-1,-1,3]),r.STATIC_DRAW);let d=r.getAttribLocation(a,`aPosition`);r.enableVertexAttribArray(d),r.vertexAttribPointer(d,2,r.FLOAT,!1,0,0);let f=Object.fromEntries([`uSize`,`uPixel`,`uTime`,`uFrom`,`uTo`,`uStart`,`uDuration`,`uX`,`uBarSize`,`uColor`].map(e=>[e,r.getUniformLocation(a,e)])),p=performance.now(),m=()=>(performance.now()-p)/1e3,h=Array.from({length:3},()=>({from:9,to:9,start:0,duration:0,reverseStart:9,factor:1})),g=0,_=0,v=!1,y=!1,S=``,O=document.createElement(`canvas`).getContext(`2d`,{willReadFrequently:!0}),k=()=>{let e=M(e=>{let t=e.firstElementChild;return t?getComputedStyle(t).backgroundColor:`transparent`});if(e===S||!O)return!1;S=e,O.clearRect(0,0,1,1),O.fillStyle=e,O.fillRect(0,0,1,1);let t=O.getImageData(0,0,1,1).data;return r.uniform4f(f.uColor??null,(t[0]??0)/255,(t[1]??0)/255,(t[2]??0)/255,(t[3]??0)/255),!0},A=e=>{if(g=0,!v||y)return;if(e+.5<_){g=requestAnimationFrame(A);return}(_===0||e>_+C)&&(_=e),_+=C;let t=m();r.uniform1f(f.uTime??null,t),r.drawArrays(r.TRIANGLES,0,3),h.some(e=>t<e.start+e.duration+w)&&(g=requestAnimationFrame(A))},j=()=>{v&&!y&&!g&&(g=requestAnimationFrame(A))};function M(n){let r=document.createElement(`div`);r.className=o(x,t.containerClassName),r.style.cssText=`position:absolute;left:0;top:0;visibility:hidden;pointer-events:none`;for(let e=0;e<3;e++){let e=document.createElement(`div`);e.className=o(b,t.barsClassName),r.append(e)}e.parentElement?.append(r);try{return n(r)}finally{r.remove()}}let N=()=>{M(t=>{let n=t.firstElementChild;if(!n)return;let i=t.clientWidth,a=t.clientHeight*2,o=window.devicePixelRatio||1,s=Math.max(1,Math.round(i*o)),c=Math.max(1,Math.round(a*o));e.width!==s&&(e.width=s),e.height!==c&&(e.height=c),e.style.width=`${i}px`,e.style.height=`${a}px`,r.viewport(0,0,e.width,e.height),r.uniform2f(f.uSize??null,i,a),r.uniform1f(f.uPixel??null,1/o);let l=getComputedStyle(n);e.style.zIndex=l.zIndex,r.uniform2f(f.uBarSize??null,parseFloat(l.width),parseFloat(l.height)),r.uniform3fv(f.uX??null,Array.from(t.children,e=>e instanceof HTMLElement?e.offsetLeft-t.offsetLeft:0))}),j()},P=()=>{for(let[e,t]of[[`uFrom`,`from`],[`uTo`,`to`],[`uStart`,`start`],[`uDuration`,`duration`]])r.uniform3fv(f[e]??null,h.map(e=>e[t]))},F=()=>n();e.addEventListener(`webglcontextlost`,F);let I=new ResizeObserver(N);e.parentElement&&I.observe(e.parentElement),window.addEventListener(`resize`,N);let L=new MutationObserver(()=>{k(),N()});for(let t=e.parentElement;t;t=t.parentElement)L.observe(t,{attributes:!0,attributeFilter:t===document.documentElement?[`class`,`style`,`data-theme`]:[`class`,`data-theme`]});P(),k();let R=u(()=>(v=!0,N(),()=>{v=!1,cancelAnimationFrame(g),g=0}));return{setTargets(e){let t=m(),n=!1;h.forEach((r,i)=>{let a=e[i];if(a===void 0||a===r.to)return;n=!0;let o=r.duration>0?D((t-r.start)/r.duration):1,s=r.from+(r.to-r.from)*o,c=t<r.start+r.duration&&r.reverseStart===a,l=c?Math.min(1,Math.abs(o*r.factor+1-r.factor)):1;h[i]={from:s,to:a,start:t,duration:.3*l,reverseStart:c?r.to:s,factor:l}}),n&&P(),n&&j()},refreshColor(){k()&&j()},dispose(){y=!0,R(),I.disconnect(),L.disconnect(),window.removeEventListener(`resize`,N),e.removeEventListener(`webglcontextlost`,F),cancelAnimationFrame(g),c()}}}var k={"--barcolor":`var(--color-dancing-bars)`};function A(e){return e.split(/\s+/).filter(e=>/^-?z-/.test(e)).join(` `)}var j=(e,t=1,n=10)=>{let r=(Math.random()*.4+.6)*t;return 10-Math.min(Math.floor(n*e/r)+1.5,n)},M=e=>{let t=(0,d.c)(5),{refProp:n,barsClassName:r}=e,i;t[0]===r?i=t[1]:(i=o(b,r),t[0]=r,t[1]=i);let a;return t[2]!==n||t[3]!==i?(a=(0,p.jsx)(`div`,{ref:n,className:i}),t[2]=n,t[3]=i,t[4]=a):a=t[4],a},N=e=>{let t=(0,d.c)(6),{children:n,style:r,className:i}=e,a;t[0]===i?a=t[1]:(a=o(x,i),t[0]=i,t[1]=a);let s;return t[2]!==n||t[3]!==r||t[4]!==a?(s=(0,p.jsx)(`div`,{className:a,style:r,children:n}),t[2]=n,t[3]=r,t[4]=a,t[5]=s):s=t[5],s};function P({className:e=``,containerClassName:t=``,barsClassName:n=``,variant:r=`default`,simulatedAmplitude:i=!1,renderer:a=`webgl`}){let s=(0,f.useRef)(null),l=(0,f.useRef)(null),d=(0,f.useRef)(null),m=(0,f.useRef)(null),h=(0,f.useRef)(null),g=(0,f.useRef)(null),_=(0,f.useRef)(null),v=(0,f.useRef)(null),b=(0,f.useRef)(void 0),[x,S]=(0,f.useState)(!1),[C,w]=(0,f.useState)(!1);(0,f.useEffect)(()=>{let e=v.current;if(a!==`webgl`||C||!e)return;let r=O(e,{barsClassName:n,containerClassName:t},()=>w(!0));return b.current=r,S(!!r),r||w(!0),()=>{r?.dispose(),b.current=void 0,S(!1)}},[a,C,n,t]);let T=(0,f.useRef)(!1),E=(0,f.useRef)(void 0),D=(0,f.useCallback)(e=>{e===0?E.current??=Date.now():E.current=void 0;let t=E.current?Date.now()-E.current>3e3:!1;_.current&&T.current!==t&&(T.current=t,_.current.style.setProperty(`--barcolor`,t?`#BD4A30`:`var(--color-dancing-bars)`),b.current?.refreshColor());let n=r===`nub`?7:10,i=j(e,2,n),a=j(e,1,n),o=j(e,2,n);if(b.current){b.current.setTargets([i,a,o]);return}s.current?.style.setProperty(`transform`,`translateY(${i}px)`),m.current?.style.setProperty(`transform`,`translateY(${i}px)`),l.current?.style.setProperty(`transform`,`translateY(${a}px)`),h.current?.style.setProperty(`transform`,`translateY(${a}px)`),d.current?.style.setProperty(`transform`,`translateY(${o}px)`),g.current?.style.setProperty(`transform`,`translateY(${o}px)`)},[r]);return(0,f.useEffect)(()=>u(i?()=>{let e=()=>D(.3+Math.random()*.5);e();let t=setInterval(e,200);return()=>clearInterval(t)}:()=>(D(0),y(e=>{let t=c.getState().micSourceMuted?0:e.microphone;D(Math.max(e.system,t))}))),[r,i,D,a,C,n,t]),(0,p.jsxs)(`div`,{className:o(`relative`,e),ref:_,style:k,children:[!x&&(0,p.jsxs)(p.Fragment,{children:[(0,p.jsxs)(N,{className:t,children:[(0,p.jsx)(M,{refProp:s,barsClassName:n}),(0,p.jsx)(M,{refProp:l,barsClassName:n}),(0,p.jsx)(M,{refProp:d,barsClassName:n})]}),(0,p.jsxs)(N,{style:{transform:`scaleY(-1)`},className:t,children:[(0,p.jsx)(M,{refProp:m,barsClassName:n}),(0,p.jsx)(M,{refProp:h,barsClassName:n}),(0,p.jsx)(M,{refProp:g,barsClassName:n})]})]}),a===`webgl`&&!C&&(0,p.jsx)(`canvas`,{ref:v,"aria-hidden":`true`,className:o(`pointer-events-none relative`,A(n)),style:{display:x?`block`:`none`}})]})}function F(e){let t=(0,d.c)(8),{className:i,barsClassName:a,containerClassName:o,variant:s,simulatedAmplitude:c}=e,u=i===void 0?``:i,f=a===void 0?``:a,m=o===void 0?``:o,g=s===void 0?`default`:s,_=c!==void 0&&c,v=l(r),y=l(n);if(v){let e;return t[0]===Symbol.for(`react.memo_cache_sentinel`)?(e=(0,p.jsx)(h,{className:`text-oats-green-500`}),t[0]=e):e=t[0],e}let b=y?`webgl`:`dom`,x;return t[1]!==b||t[2]!==f||t[3]!==u||t[4]!==m||t[5]!==_||t[6]!==g?(x=(0,p.jsx)(P,{className:u,barsClassName:f,containerClassName:m,variant:g,simulatedAmplitude:_,renderer:b},b),t[1]=b,t[2]=f,t[3]=u,t[4]=m,t[5]=_,t[6]=g,t[7]=x):x=t[7],x}export{u as a,m as i,y as n,h as r,F as t};
//# sourceMappingURL=RandomAudioVisualizer-B_NgdYL1.js.map