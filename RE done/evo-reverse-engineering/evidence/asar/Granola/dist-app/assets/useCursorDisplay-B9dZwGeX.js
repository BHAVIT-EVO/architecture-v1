try{let e=typeof window<`u`?window:typeof global<`u`?global:typeof globalThis<`u`?globalThis:typeof self<`u`?self:{},t=new e.Error().stack;t&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[t]=`51f30ff4-dee0-4682-b0b2-6f623af8d7c9`,e._sentryDebugIdIdentifier=`sentry-dbid-51f30ff4-dee0-4682-b0b2-6f623af8d7c9`)}catch{}import"./_sentry-release-injection-file-DZROVGkF.js";import{a as e}from"./rolldown-runtime-CMh1wmV-.js";import{t}from"./react-DLCF0spK.js";import{t as n}from"./compiler-runtime-DK4A0O2g.js";import{t as r}from"./jsx-runtime-DwLIIa17.js";import{g as i,n as a}from"./useQuery-BifTMEuo.js";import{i as o,n as s,r as c}from"./queryClient-BnmiLCuI.js";import{H as l,N as u,P as d,S as f,U as p,_ as m,k as h,x as g}from"./proxy-DLjYPJLW.js";import{t as _}from"./use-reduced-motion-Bsith9jb.js";import{t as v}from"./buddy-Bw9qwxH9.js";var y=class extends i{constructor(e,t){super(e,t)}bindMethods(){super.bindMethods(),this.fetchNextPage=this.fetchNextPage.bind(this),this.fetchPreviousPage=this.fetchPreviousPage.bind(this)}setOptions(e,t){super.setOptions({...e,behavior:o()},t)}getOptimisticResult(e){return e.behavior=o(),super.getOptimisticResult(e)}fetchNextPage(e){return this.fetch({...e,meta:{fetchMore:{direction:`forward`}}})}fetchPreviousPage(e){return this.fetch({...e,meta:{fetchMore:{direction:`backward`}}})}createResult(e,t){let{state:n}=e,r=super.createResult(e,t),{isFetching:i,isRefetching:a,isError:o,isRefetchError:l}=r,u=n.fetchMeta?.fetchMore?.direction,d=o&&u===`forward`,f=i&&u===`forward`,p=o&&u===`backward`,m=i&&u===`backward`;return{...r,fetchNextPage:this.fetchNextPage,fetchPreviousPage:this.fetchPreviousPage,hasNextPage:s(t,n.data),hasPreviousPage:c(t,n.data),isFetchNextPageError:d,isFetchingNextPage:f,isFetchPreviousPageError:p,isFetchingPreviousPage:m,isRefetchError:l&&!d&&!p,isRefetching:a&&!f&&!m}}};function b(e,t){return a(e,y,t)}function x(...e){let t=!Array.isArray(e[0]),n=t?0:-1,r=e[0+n],i=e[1+n],a=e[2+n],o=e[3+n],s=h(i,a,o);return t?s(r):s}var S=e(t(),1);function C(e){let t=p(()=>f(e)),{isStatic:n}=(0,S.useContext)(m);if(n){let[,n]=(0,S.useState)(e);(0,S.useEffect)(()=>t.on(`change`,n),[])}return t}function w(e,t){let n=C(t()),r=()=>n.set(t());return r(),l(()=>{let t=()=>d.preRender(r,!1,!0),n=e.map(e=>e.on(`change`,t));return()=>{n.forEach(e=>e()),u(r)}}),n}function T(e){g.current=[],e();let t=w(g.current,e);return g.current=void 0,t}function E(e,t,n,r){if(typeof e==`function`)return T(e);let i=typeof t==`function`?t:x(t,n,r);return Array.isArray(e)?D(e,i):D([e],([e])=>i(e))}function D(e,t){let n=p(()=>[]);return w(e,()=>{n.length=0;let r=e.length;for(let t=0;t<r;t++)n[t]=e[t].get();return t(n)})}var O=n(),k=`#version 300 es
in vec2 aPosition;
out vec2 vPosition;
void main() {
  vPosition = aPosition;
  gl_Position = vec4(aPosition, 0.0, 1.0);
}`,A=`#version 300 es
precision highp float;
in vec2 vPosition;
out vec4 outColor;
uniform sampler2D uImage;
uniform float uOpen;
uniform float uBlink;
uniform float uRoll;
uniform float uLift;
uniform float uFocus;
uniform vec2 uStretch;

const float SIZE = 1024.0;
const float PADDING = 1.08;
const vec2 BALL = vec2(510.0, 498.0);
const vec2 LEFT_EYE = vec2(372.0, 328.0);
const vec2 RIGHT_EYE = vec2(645.0, 328.0);
const vec2 EYE_RADIUS = vec2(92.0, 112.0);
const float EYELID_REACH = 170.0;
const vec2 SMILE_CENTER = vec2(512.0, 380.0);
const float SMILE_RADIUS = 337.0;
const float SMILE_HALF_WIDTH = 300.0;

// Squashes the printed eye toward its center and stretches the surrounding
// skin to fill the gap. Monotonic per column, so no seams or doubled ink.
vec2 blink(vec2 p, vec2 eye) {
  vec2 d = p - eye;
  float column = 1.0 - pow(d.x / EYE_RADIUS.x, 2.0);
  if (column <= 0.0 || abs(d.y) >= EYELID_REACH) return p;
  float eyeHalf = EYE_RADIUS.y * sqrt(column);
  float lidHalf = eyeHalf * max(1.0 - uBlink, 0.04);
  float y = abs(d.y);
  float source = y < lidHalf
    ? y * eyeHalf / lidHalf
    : eyeHalf + (y - lidHalf) * (EYELID_REACH - eyeHalf) / (EYELID_REACH - lidHalf);
  return vec2(p.x, eye.y + sign(d.y) * mix(y, source, smoothstep(0.0, 0.35, column)));
}

vec2 mouth(vec2 p) {
  vec2 v = p - SMILE_CENTER;
  float t = clamp(v.x / SMILE_HALF_WIDTH, -1.0, 1.0);
  float bulge = (1.0 - t * t) * smoothstep(50.0, 150.0, v.y);
  float radial = length(v) - SMILE_RADIUS;
  // Magnifying around the smile's centerline thickens the printed stroke.
  // The falloff keeps the mapping monotonic, so nothing folds or tears.
  float zoom = 1.0 / (1.0 + uOpen * 1.2 * bulge);
  float weight = 1.0 - smoothstep(40.0, 130.0, abs(radial));
  float source = radial * mix(1.0, zoom, weight);
  return SMILE_CENTER + normalize(v) * (SMILE_RADIUS + source);
}

void main() {
  vec2 p = SIZE * 0.5 + vPosition * vec2(1.0, -1.0) * SIZE * 0.5 * PADDING;
  float cr = cos(uRoll);
  float sr = sin(uRoll);
  p = BALL + mat2(cr, -sr, sr, cr) * (p - BALL) / uStretch + vec2(0.0, uLift);
  float eyeDistance = min(length((p - LEFT_EYE) / EYE_RADIUS), length((p - RIGHT_EYE) / EYE_RADIUS));
  p.y -= uFocus * 55.0 * (1.0 - smoothstep(1.0, 1.8, eyeDistance));
  p = blink(blink(mouth(p), LEFT_EYE), RIGHT_EYE);
  vec2 uv = p / SIZE;
  if (any(lessThan(uv, vec2(0.0))) || any(greaterThan(uv, vec2(1.0)))) {
    outColor = vec4(0.0);
    return;
  }
  // Masks stray pixels along the source image's border.
  float ball = length((p - BALL) / vec2(468.0, 478.0));
  outColor = texture(uImage, uv) * (1.0 - smoothstep(1.015, 1.035, ball));
}`;function j(e,t,n){let r=e.createShader(t);return r?(e.shaderSource(r,n),e.compileShader(r),e.getShaderParameter(r,e.COMPILE_STATUS)?r:(console.warn(`assistant-avatar-shader-failed`,e.getShaderInfoLog(r)),e.deleteShader(r),null)):null}var M=.16;function N(e,{image:t,focused:n,dragging:r,loading:i,level:a,reducedMotion:o,onFailed:s}){let c=e.getContext(`webgl2`,{premultipliedAlpha:!0});if(!c)return null;let l=j(c,c.VERTEX_SHADER,k),u=j(c,c.FRAGMENT_SHADER,A),d=c.createProgram();if(!l||!u||!d||(c.attachShader(d,l),c.attachShader(d,u),c.linkProgram(d),!c.getProgramParameter(d,c.LINK_STATUS)))return null;c.useProgram(d);let f=c.createBuffer();c.bindBuffer(c.ARRAY_BUFFER,f),c.bufferData(c.ARRAY_BUFFER,new Float32Array([-1,-1,1,-1,-1,1,1,1]),c.STATIC_DRAW);let p=c.getAttribLocation(d,`aPosition`);c.enableVertexAttribArray(p),c.vertexAttribPointer(p,2,c.FLOAT,!1,0,0);let m=e=>c.getUniformLocation(d,e),h=m(`uFocus`),g=m(`uOpen`),_=m(`uBlink`),v=m(`uRoll`),y=m(`uLift`),b=m(`uStretch`),x=c.createTexture(),S=a&&!o?1:0,C=0,w=0,T=0,E=0,D=-1,O=o?1/0:performance.now()/1e3+1.5+Math.random()*2,N=0,P,F=!1,I=!1,L=t=>{let s=t/1e3,l=Math.min((t-E)/1e3,.05);E=t;let u=i?.()&&!o?1:0;T+=(u-T)*(1-Math.exp(-l*8));let d=T*Math.sin(s*3),f=T*Math.sin(s*6),p=e.ownerDocument.defaultView?.devicePixelRatio??1,m=Math.min(1024,Math.ceil(e.clientWidth*p*2));m>0&&e.width!==m&&(e.width=m,e.height=m),w+=(+!!n?.()-w)*.12,c.uniform1f(h,w);let x=+!!r?.(),k=x?.9:Math.min(1,(a?.()??0)*1.2);C+=(k-C)*(k>C?.5:.15),s>=O&&(D=s,O=s+(Math.random()<.15?.3:2.5+Math.random()*3.5));let A=(s-D)/M,j=A>=0&&A<1?Math.sin(Math.PI*A):0,F=1+S*.008*Math.sin(s*1.7);c.viewport(0,0,e.width,e.height),c.clearColor(0,0,0,0),c.clear(c.COLOR_BUFFER_BIT),c.uniform1f(g,C),c.uniform1f(_,x?.65:j),c.uniform1f(v,S*(x*(.12+.06*Math.sin(s*12))+.025*Math.sin(s*.8)+.03*C*Math.sin(s*5.3)+w*.008*Math.sin(s*8))+d*.16),c.uniform1f(y,S*(6*C+4*Math.sin(s*1.7)+w*(-15+4*Math.sin(s*8)))+f*16),c.uniform2f(b,F*(1-S*.02*C),F*(1+S*.035*C)),c.drawArrays(c.TRIANGLE_STRIP,0,4),a||u||T>.001||A<1?N=requestAnimationFrame(L):Number.isFinite(O)&&(P=setTimeout(()=>N=requestAnimationFrame(L),(O-s)*1e3))},R=()=>{!F||I||(cancelAnimationFrame(N),clearTimeout(P),N=requestAnimationFrame(L))},z=new ResizeObserver(R);z.observe(e);let B=new Image;B.onload=()=>{I||(c.bindTexture(c.TEXTURE_2D,x),c.pixelStorei(c.UNPACK_PREMULTIPLY_ALPHA_WEBGL,!0),c.texImage2D(c.TEXTURE_2D,0,c.RGBA,c.RGBA,c.UNSIGNED_BYTE,B),c.generateMipmap(c.TEXTURE_2D),c.texParameteri(c.TEXTURE_2D,c.TEXTURE_MIN_FILTER,c.LINEAR_MIPMAP_LINEAR),c.texParameteri(c.TEXTURE_2D,c.TEXTURE_MAG_FILTER,c.LINEAR),c.texParameteri(c.TEXTURE_2D,c.TEXTURE_WRAP_S,c.CLAMP_TO_EDGE),c.texParameteri(c.TEXTURE_2D,c.TEXTURE_WRAP_T,c.CLAMP_TO_EDGE),F=!0,R())},B.onerror=()=>{I||s()},B.src=t;let V=e=>{e.preventDefault(),cancelAnimationFrame(N),clearTimeout(P),s()};return e.addEventListener(`webglcontextlost`,V),{dispose:()=>{I=!0,z.disconnect(),cancelAnimationFrame(N),clearTimeout(P),e.removeEventListener(`webglcontextlost`,V),c.deleteTexture(x),c.deleteBuffer(f),c.deleteProgram(d),c.deleteShader(l),c.deleteShader(u)},redraw:R}}var P=r();function F(e){let t=(0,O.c)(17),{level:n,focused:r,dragging:i,loading:a,className:o}=e,s=r!==void 0&&r,c=i!==void 0&&i,l=a!==void 0&&a,u=_(),d=(0,S.useRef)(null),f=(0,S.useRef)(l),p,m;t[0]===l?(p=t[1],m=t[2]):(p=()=>{f.current=l,d.current?.redraw()},m=[l],t[0]=l,t[1]=p,t[2]=m),(0,S.useEffect)(p,m);let h=(0,S.useRef)(c),g,y;t[3]===c?(g=t[4],y=t[5]):(g=()=>{h.current=c},y=[c],t[3]=c,t[4]=g,t[5]=y),(0,S.useEffect)(g,y);let b=(0,S.useRef)(s),x,C;t[6]===s?(x=t[7],C=t[8]):(x=()=>{b.current=s},C=[s],t[6]=s,t[7]=x,t[8]=C),(0,S.useEffect)(x,C);let w=(0,S.useRef)(null),[T,E]=(0,S.useState)(!1),D,k;if(t[9]!==n||t[10]!==u?(D=()=>{let e=w.current;if(!e)return;let t=N(e,{image:v,focused:()=>b.current,dragging:()=>h.current,loading:()=>f.current,level:n?()=>n.get():void 0,reducedMotion:!!u,onFailed:()=>E(!0)});return d.current=t,t||E(!0),()=>{t?.dispose(),d.current=null}},k=[n,u],t[9]=n,t[10]=u,t[11]=D,t[12]=k):(D=t[11],k=t[12]),(0,S.useEffect)(D,k),T){let e=`object-contain ${o}`,n;return t[13]===e?n=t[14]:(n=(0,P.jsx)(`img`,{src:v,alt:``,className:e}),t[13]=e,t[14]=n),n}let A;return t[15]===o?A=t[16]:(A=(0,P.jsx)(`canvas`,{ref:w,"aria-hidden":`true`,className:o}),t[15]=o,t[16]=A),A}function I(e){let t=(0,O.c)(4),[n,r]=(0,S.useState)(null),{onCursorDisplayChange:i}=window.electron,a,o;if(t[0]===e?(a=t[1],o=t[2]):(a=()=>{if(!e||!i)return;let t=i(r);return()=>{t(),r(null)}},o=[e,i],t[0]=e,t[1]=a,t[2]=o),(0,S.useEffect)(a,o),i)return n;let{availLeft:s,availTop:c,availWidth:l,availHeight:u}=window.screen,d;return t[3]===Symbol.for(`react.memo_cache_sentinel`)?(d={workArea:{x:s,y:c,width:l,height:u}},t[3]=d):d=t[3],d}export{b as a,C as i,F as n,E as r,I as t};
//# sourceMappingURL=useCursorDisplay-B9dZwGeX.js.map