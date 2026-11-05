try{let e=typeof window<`u`?window:typeof global<`u`?global:typeof globalThis<`u`?globalThis:typeof self<`u`?self:{},t=new e.Error().stack;t&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[t]=`b7594995-de5b-41ca-9072-f82a15c75f30`,e._sentryDebugIdIdentifier=`sentry-dbid-b7594995-de5b-41ca-9072-f82a15c75f30`)}catch{}import"./_sentry-release-injection-file-DZROVGkF.js";import{a as e}from"./rolldown-runtime-CMh1wmV-.js";import{t}from"./react-DLCF0spK.js";import{i as n}from"./object-B8S4EioD.js";import{t as r}from"./compiler-runtime-DK4A0O2g.js";import{t as i}from"./react-dom-D1Ad4Yfc.js";import{t as a}from"./jsx-runtime-DwLIIa17.js";var o=r(),s=e(t(),1),c=e(i(),1),l=a(),u=`@keyframes styles-module__popupEnter___AuQDN {
  from {
    opacity: 0;
    transform: translateX(-50%) scale(0.95) translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateX(-50%) scale(1) translateY(0);
  }
}
@keyframes styles-module__popupExit___JJKQX {
  from {
    opacity: 1;
    transform: translateX(-50%) scale(1) translateY(0);
  }
  to {
    opacity: 0;
    transform: translateX(-50%) scale(0.95) translateY(4px);
  }
}
@keyframes styles-module__shake___jdbWe {
  0%, 100% {
    transform: translateX(-50%) scale(1) translateY(0) translateX(0);
  }
  20% {
    transform: translateX(-50%) scale(1) translateY(0) translateX(-3px);
  }
  40% {
    transform: translateX(-50%) scale(1) translateY(0) translateX(3px);
  }
  60% {
    transform: translateX(-50%) scale(1) translateY(0) translateX(-2px);
  }
  80% {
    transform: translateX(-50%) scale(1) translateY(0) translateX(2px);
  }
}
.styles-module__popup___IhzrD {
  position: fixed;
  transform: translateX(-50%);
  width: 280px;
  padding: 0.75rem 1rem 14px;
  background: #1a1a1a;
  border-radius: 16px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.3), 0 0 0 1px rgba(255, 255, 255, 0.08);
  cursor: default;
  z-index: 100001;
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  will-change: transform, opacity;
  opacity: 0;
}
.styles-module__popup___IhzrD.styles-module__enter___L7U7N {
  animation: styles-module__popupEnter___AuQDN 0.2s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
}
.styles-module__popup___IhzrD.styles-module__entered___COX-w {
  opacity: 1;
  transform: translateX(-50%) scale(1) translateY(0);
}
.styles-module__popup___IhzrD.styles-module__exit___5eGjE {
  animation: styles-module__popupExit___JJKQX 0.15s ease-in forwards;
}
.styles-module__popup___IhzrD.styles-module__entered___COX-w.styles-module__shake___jdbWe {
  animation: styles-module__shake___jdbWe 0.25s ease-out;
}

.styles-module__header___wWsSi {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.5rem;
}

.styles-module__element___fTV2z {
  font-size: 0.75rem;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.65);
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.styles-module__headerToggle___WpW0b {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  background: none;
  border: none;
  padding: 0;
  cursor: pointer;
  flex: 1;
  min-width: 0;
  text-align: left;
}
.styles-module__headerToggle___WpW0b .styles-module__element___fTV2z {
  flex: 1;
}

.styles-module__chevron___ZZJlR {
  color: rgba(255, 255, 255, 0.5);
  transition: transform 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  flex-shrink: 0;
}
.styles-module__chevron___ZZJlR.styles-module__expanded___2Hxgv {
  transform: rotate(90deg);
}

.styles-module__stylesWrapper___pnHgy {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}
.styles-module__stylesWrapper___pnHgy.styles-module__expanded___2Hxgv {
  grid-template-rows: 1fr;
}

.styles-module__stylesInner___YYZe2 {
  overflow: hidden;
}

.styles-module__stylesBlock___VfQKn {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 0.375rem;
  padding: 0.5rem 0.625rem;
  margin-bottom: 0.5rem;
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  font-size: 0.6875rem;
  line-height: 1.5;
}

.styles-module__styleLine___1YQiD {
  color: rgba(255, 255, 255, 0.85);
  word-break: break-word;
}

.styles-module__styleProperty___84L1i {
  color: #c792ea;
}

.styles-module__styleValue___q51-h {
  color: rgba(255, 255, 255, 0.85);
}

.styles-module__timestamp___Dtpsv {
  font-size: 0.625rem;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.35);
  font-variant-numeric: tabular-nums;
  margin-left: 0.5rem;
  flex-shrink: 0;
}

.styles-module__quote___mcMmQ {
  font-size: 0.6875rem;
  font-style: italic;
  color: rgba(255, 255, 255, 0.5);
  margin-bottom: 0.5rem;
  padding: 0.4rem 0.5rem;
  background: rgba(255, 255, 255, 0.05);
  border-radius: 0.25rem;
  line-height: 1.45;
}

.styles-module__textarea___jrSae {
  width: 100%;
  padding: 0.5rem 0.625rem;
  font-size: 0.8125rem;
  font-family: inherit;
  background: rgba(255, 255, 255, 0.05);
  color: #fff;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 8px;
  resize: none;
  outline: none;
  transition: border-color 0.15s ease;
}
.styles-module__textarea___jrSae:focus {
  border-color: #3c82f7;
}
.styles-module__textarea___jrSae.styles-module__green___99l3h:focus {
  border-color: #34C759;
}
.styles-module__textarea___jrSae::placeholder {
  color: rgba(255, 255, 255, 0.35);
}
.styles-module__textarea___jrSae::-webkit-scrollbar {
  width: 6px;
}
.styles-module__textarea___jrSae::-webkit-scrollbar-track {
  background: transparent;
}
.styles-module__textarea___jrSae::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 3px;
}

.styles-module__actions___D6x3f {
  display: flex;
  justify-content: flex-end;
  gap: 0.375rem;
  margin-top: 0.5rem;
}

.styles-module__cancel___hRjnL,
.styles-module__submit___K-mIR {
  padding: 0.4rem 0.875rem;
  font-size: 0.75rem;
  font-weight: 500;
  border-radius: 1rem;
  border: none;
  cursor: pointer;
  transition: background-color 0.15s ease, color 0.15s ease, opacity 0.15s ease;
}

.styles-module__cancel___hRjnL {
  background: transparent;
  color: rgba(255, 255, 255, 0.5);
}
.styles-module__cancel___hRjnL:hover {
  background: rgba(255, 255, 255, 0.1);
  color: rgba(255, 255, 255, 0.8);
}

.styles-module__submit___K-mIR {
  color: white;
}
.styles-module__submit___K-mIR:hover:not(:disabled) {
  filter: brightness(0.9);
}
.styles-module__submit___K-mIR:disabled {
  cursor: not-allowed;
}

.styles-module__light___6AaSQ.styles-module__popup___IhzrD {
  background: #fff;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.12), 0 0 0 1px rgba(0, 0, 0, 0.06);
}
.styles-module__light___6AaSQ .styles-module__element___fTV2z {
  color: rgba(0, 0, 0, 0.6);
}
.styles-module__light___6AaSQ .styles-module__timestamp___Dtpsv {
  color: rgba(0, 0, 0, 0.4);
}
.styles-module__light___6AaSQ .styles-module__chevron___ZZJlR {
  color: rgba(0, 0, 0, 0.4);
}
.styles-module__light___6AaSQ .styles-module__stylesBlock___VfQKn {
  background: rgba(0, 0, 0, 0.03);
}
.styles-module__light___6AaSQ .styles-module__styleLine___1YQiD {
  color: rgba(0, 0, 0, 0.75);
}
.styles-module__light___6AaSQ .styles-module__styleProperty___84L1i {
  color: #7c3aed;
}
.styles-module__light___6AaSQ .styles-module__styleValue___q51-h {
  color: rgba(0, 0, 0, 0.75);
}
.styles-module__light___6AaSQ .styles-module__quote___mcMmQ {
  color: rgba(0, 0, 0, 0.55);
  background: rgba(0, 0, 0, 0.04);
}
.styles-module__light___6AaSQ .styles-module__textarea___jrSae {
  background: rgba(0, 0, 0, 0.03);
  color: #1a1a1a;
  border-color: rgba(0, 0, 0, 0.12);
}
.styles-module__light___6AaSQ .styles-module__textarea___jrSae::placeholder {
  color: rgba(0, 0, 0, 0.4);
}
.styles-module__light___6AaSQ .styles-module__textarea___jrSae::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.15);
}
.styles-module__light___6AaSQ .styles-module__cancel___hRjnL {
  color: rgba(0, 0, 0, 0.5);
}
.styles-module__light___6AaSQ .styles-module__cancel___hRjnL:hover {
  background: rgba(0, 0, 0, 0.06);
  color: rgba(0, 0, 0, 0.75);
}`,d={popup:`styles-module__popup___IhzrD`,enter:`styles-module__enter___L7U7N`,popupEnter:`styles-module__popupEnter___AuQDN`,entered:`styles-module__entered___COX-w`,exit:`styles-module__exit___5eGjE`,popupExit:`styles-module__popupExit___JJKQX`,shake:`styles-module__shake___jdbWe`,header:`styles-module__header___wWsSi`,element:`styles-module__element___fTV2z`,headerToggle:`styles-module__headerToggle___WpW0b`,chevron:`styles-module__chevron___ZZJlR`,expanded:`styles-module__expanded___2Hxgv`,stylesWrapper:`styles-module__stylesWrapper___pnHgy`,stylesInner:`styles-module__stylesInner___YYZe2`,stylesBlock:`styles-module__stylesBlock___VfQKn`,styleLine:`styles-module__styleLine___1YQiD`,styleProperty:`styles-module__styleProperty___84L1i`,styleValue:`styles-module__styleValue___q51-h`,timestamp:`styles-module__timestamp___Dtpsv`,quote:`styles-module__quote___mcMmQ`,textarea:`styles-module__textarea___jrSae`,green:`styles-module__green___99l3h`,actions:`styles-module__actions___D6x3f`,cancel:`styles-module__cancel___hRjnL`,submit:`styles-module__submit___K-mIR`,light:`styles-module__light___6AaSQ`};if(typeof document<`u`){let e=document.getElementById(`feedback-tool-styles-annotation-popup-css-styles`);e||(e=document.createElement(`style`),e.id=`feedback-tool-styles-annotation-popup-css-styles`,e.textContent=u,document.head.appendChild(e))}var f=d,p=(0,s.forwardRef)(function({element:e,timestamp:t,selectedText:n,placeholder:r=`What should change?`,initialValue:i=``,submitLabel:a=`Add`,onSubmit:o,onCancel:c,style:u,accentColor:d=`#3c82f7`,isExiting:p=!1,lightMode:m=!1,computedStyles:h},ee){let[g,te]=(0,s.useState)(i),[_,ne]=(0,s.useState)(!1),[v,y]=(0,s.useState)(`initial`),[re,ie]=(0,s.useState)(!1),[ae,oe]=(0,s.useState)(!1),b=(0,s.useRef)(null),x=(0,s.useRef)(null);(0,s.useEffect)(()=>{p&&v!==`exit`&&y(`exit`)},[p,v]),(0,s.useEffect)(()=>{requestAnimationFrame(()=>{y(`enter`)});let e=setTimeout(()=>{y(`entered`)},200),t=setTimeout(()=>{let e=b.current;e&&(e.focus(),e.selectionStart=e.selectionEnd=e.value.length,e.scrollTop=e.scrollHeight)},50);return()=>{clearTimeout(e),clearTimeout(t)}},[]);let S=(0,s.useCallback)(()=>{ne(!0),setTimeout(()=>{ne(!1),b.current?.focus()},250)},[]);(0,s.useImperativeHandle)(ee,()=>({shake:S}),[S]);let C=(0,s.useCallback)(()=>{y(`exit`),setTimeout(()=>{c()},150)},[c]),w=(0,s.useCallback)(()=>{g.trim()&&o(g.trim())},[g,o]),se=(0,s.useCallback)(e=>{e.key===`Enter`&&!e.shiftKey&&(e.preventDefault(),w()),e.key===`Escape`&&C()},[w,C]),T=[f.popup,m?f.light:``,v===`enter`?f.enter:``,v===`entered`?f.entered:``,v===`exit`?f.exit:``,_?f.shake:``].filter(Boolean).join(` `);return(0,l.jsxs)(`div`,{ref:x,className:T,"data-annotation-popup":!0,style:u,onClick:e=>e.stopPropagation(),children:[(0,l.jsxs)(`div`,{className:f.header,children:[h&&Object.keys(h).length>0?(0,l.jsxs)(`button`,{className:f.headerToggle,onClick:()=>{let e=ae;oe(!ae),e&&setTimeout(()=>b.current?.focus(),0)},type:`button`,children:[(0,l.jsx)(`svg`,{className:`${f.chevron} ${ae?f.expanded:``}`,width:`14`,height:`14`,viewBox:`0 0 14 14`,fill:`none`,xmlns:`http://www.w3.org/2000/svg`,children:(0,l.jsx)(`path`,{d:`M5.5 10.25L9 7.25L5.75 4`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,strokeLinejoin:`round`})}),(0,l.jsx)(`span`,{className:f.element,children:e})]}):(0,l.jsx)(`span`,{className:f.element,children:e}),t&&(0,l.jsx)(`span`,{className:f.timestamp,children:t})]}),h&&Object.keys(h).length>0&&(0,l.jsx)(`div`,{className:`${f.stylesWrapper} ${ae?f.expanded:``}`,children:(0,l.jsx)(`div`,{className:f.stylesInner,children:(0,l.jsx)(`div`,{className:f.stylesBlock,children:Object.entries(h).map(([e,t])=>(0,l.jsxs)(`div`,{className:f.styleLine,children:[(0,l.jsx)(`span`,{className:f.styleProperty,children:e.replace(/([A-Z])/g,`-$1`).toLowerCase()}),`: `,(0,l.jsx)(`span`,{className:f.styleValue,children:t}),`;`]},e))})})}),n&&(0,l.jsxs)(`div`,{className:f.quote,children:[`“`,n.slice(0,80),n.length>80?`...`:``,`”`]}),(0,l.jsx)(`textarea`,{ref:b,className:f.textarea,style:{borderColor:re?d:void 0},placeholder:r,value:g,onChange:e=>te(e.target.value),onFocus:()=>ie(!0),onBlur:()=>ie(!1),rows:2,onKeyDown:se}),(0,l.jsxs)(`div`,{className:f.actions,children:[(0,l.jsx)(`button`,{className:f.cancel,onClick:C,children:`Cancel`}),(0,l.jsx)(`button`,{className:f.submit,style:{backgroundColor:d,opacity:g.trim()?1:.4},onClick:w,disabled:!g.trim(),children:a})]})]})}),m=({size:e=16})=>(0,l.jsx)(`svg`,{width:e,height:e,viewBox:`0 0 16 16`,fill:`none`,children:(0,l.jsx)(`path`,{d:`M4 4l8 8M12 4l-8 8`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`})}),h=({size:e=16})=>(0,l.jsx)(`svg`,{width:e,height:e,viewBox:`0 0 16 16`,fill:`none`,children:(0,l.jsx)(`path`,{d:`M8 3v10M3 8h10`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`})}),ee=({size:e=24,style:t={}})=>(0,l.jsxs)(`svg`,{width:e,height:e,viewBox:`0 0 24 24`,fill:`none`,style:t,children:[(0,l.jsxs)(`g`,{clipPath:`url(#clip0_list_sparkle)`,children:[(0,l.jsx)(`path`,{d:`M11.5 12L5.5 12`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,strokeLinejoin:`round`}),(0,l.jsx)(`path`,{d:`M18.5 6.75L5.5 6.75`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,strokeLinejoin:`round`}),(0,l.jsx)(`path`,{d:`M9.25 17.25L5.5 17.25`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,strokeLinejoin:`round`}),(0,l.jsx)(`path`,{d:`M16 12.75L16.5179 13.9677C16.8078 14.6494 17.3506 15.1922 18.0323 15.4821L19.25 16L18.0323 16.5179C17.3506 16.8078 16.8078 17.3506 16.5179 18.0323L16 19.25L15.4821 18.0323C15.1922 17.3506 14.6494 16.8078 13.9677 16.5179L12.75 16L13.9677 15.4821C14.6494 15.1922 15.1922 14.6494 15.4821 13.9677L16 12.75Z`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinejoin:`round`})]}),(0,l.jsx)(`defs`,{children:(0,l.jsx)(`clipPath`,{id:`clip0_list_sparkle`,children:(0,l.jsx)(`rect`,{width:`24`,height:`24`,fill:`white`})})})]}),g=({size:e=20})=>(0,l.jsx)(`svg`,{width:e,height:e,viewBox:`0 0 20 20`,fill:`none`,children:(0,l.jsx)(`path`,{d:`M10 3.33301C13.6816 3.33318 16.6658 6.31838 16.666 10C16.6658 13.6816 13.6816 16.6658 10 16.666C6.31836 16.6658 3.33318 13.6816 3.33301 10C3.33321 6.31838 6.31838 3.33319 10 3.33301ZM10 4.58301C7.00873 4.58319 4.58321 7.00874 4.58301 10C4.58318 12.9913 7.00872 15.4158 10 15.416C12.9913 15.4158 15.4158 12.9913 15.416 10C15.4158 7.00874 12.9912 4.58318 10 4.58301ZM10.0059 12.3955C10.3509 12.3955 10.6307 12.6755 10.6309 13.0205C10.6309 13.3657 10.351 13.6455 10.0059 13.6455H10C9.65482 13.6455 9.375 13.3657 9.375 13.0205C9.3752 12.6755 9.65494 12.3955 10 12.3955H10.0059ZM8.7168 6.6875C9.21305 6.39589 9.79695 6.28941 10.3643 6.38672C10.9314 6.48413 11.4458 6.77949 11.8164 7.21973C12.1868 7.65987 12.3903 8.21677 12.3896 8.79199L12.3818 8.96191C12.3064 9.79544 11.6881 10.3569 11.2354 10.667C11.0017 10.827 10.7702 10.9462 10.5898 11.0312C10.5068 11.2788 10.2755 11.4589 10 11.459C9.65518 11.4588 9.37535 11.1788 9.375 10.834V10.6045C9.375 10.3374 9.54513 10.0992 9.79785 10.0127C9.79944 10.0121 9.80223 10.011 9.80566 10.0098C9.81443 10.0066 9.82943 10.0017 9.84863 9.99414C9.88759 9.97889 9.94594 9.95512 10.0166 9.92285C10.1601 9.85732 10.3469 9.76066 10.5293 9.63574C10.9282 9.36253 11.1394 9.07213 11.1396 8.79199V8.79102C11.1401 8.51071 11.0408 8.23891 10.8604 8.02441C10.6798 7.80996 10.4286 7.66661 10.1523 7.61914C9.87611 7.57183 9.59223 7.62367 9.35059 7.76562C9.10896 7.90763 8.92515 8.1302 8.83203 8.39453C8.71753 8.71983 8.36054 8.89136 8.03516 8.77734C7.70969 8.66285 7.53811 8.30597 7.65234 7.98047C7.84336 7.43747 8.22055 6.97917 8.7168 6.6875Z`,fill:`currentColor`})}),te=({size:e=14})=>(0,l.jsxs)(`svg`,{width:e,height:e,viewBox:`0 0 14 14`,fill:`none`,children:[(0,l.jsx)(`style`,{children:`
      @keyframes checkDraw {
        0% {
          stroke-dashoffset: 12;
        }
        100% {
          stroke-dashoffset: 0;
        }
      }
      @keyframes checkBounce {
        0% {
          transform: scale(0.5);
          opacity: 0;
        }
        50% {
          transform: scale(1.12);
          opacity: 1;
        }
        75% {
          transform: scale(0.95);
        }
        100% {
          transform: scale(1);
        }
      }
      .check-path-animated {
        stroke-dasharray: 12;
        stroke-dashoffset: 0;
        transform-origin: center;
        animation: checkDraw 0.18s ease-out, checkBounce 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
      }
    `}),(0,l.jsx)(`path`,{className:`check-path-animated`,d:`M3.9375 7L6.125 9.1875L10.5 4.8125`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,strokeLinejoin:`round`})]}),_=({size:e=24,copied:t=!1})=>(0,l.jsxs)(`svg`,{width:e,height:e,viewBox:`0 0 24 24`,fill:`none`,children:[(0,l.jsx)(`style`,{children:`
      .copy-icon, .check-icon {
        transition: opacity 0.2s ease, transform 0.2s ease;
      }
    `}),(0,l.jsxs)(`g`,{className:`copy-icon`,style:{opacity:+!t,transform:t?`scale(0.8)`:`scale(1)`,transformOrigin:`center`},children:[(0,l.jsx)(`path`,{d:`M4.75 11.25C4.75 10.4216 5.42157 9.75 6.25 9.75H12.75C13.5784 9.75 14.25 10.4216 14.25 11.25V17.75C14.25 18.5784 13.5784 19.25 12.75 19.25H6.25C5.42157 19.25 4.75 18.5784 4.75 17.75V11.25Z`,stroke:`currentColor`,strokeWidth:`1.5`}),(0,l.jsx)(`path`,{d:`M17.25 14.25H17.75C18.5784 14.25 19.25 13.5784 19.25 12.75V6.25C19.25 5.42157 18.5784 4.75 17.75 4.75H11.25C10.4216 4.75 9.75 5.42157 9.75 6.25V6.75`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`})]}),(0,l.jsxs)(`g`,{className:`check-icon`,style:{opacity:+!!t,transform:t?`scale(1)`:`scale(0.8)`,transformOrigin:`center`},children:[(0,l.jsx)(`path`,{d:`M12 20C7.58172 20 4 16.4182 4 12C4 7.58172 7.58172 4 12 4C16.4182 4 20 7.58172 20 12C20 16.4182 16.4182 20 12 20Z`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,strokeLinejoin:`round`}),(0,l.jsx)(`path`,{d:`M15 10L11 14.25L9.25 12.25`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,strokeLinejoin:`round`})]})]}),ne=({size:e=24,isOpen:t=!0})=>(0,l.jsxs)(`svg`,{width:e,height:e,viewBox:`0 0 24 24`,fill:`none`,children:[(0,l.jsx)(`style`,{children:`
      .eye-open, .eye-closed {
        transition: opacity 0.2s ease;
      }
    `}),(0,l.jsxs)(`g`,{className:`eye-open`,style:{opacity:+!!t},children:[(0,l.jsx)(`path`,{d:`M3.91752 12.7539C3.65127 12.2996 3.65037 11.7515 3.9149 11.2962C4.9042 9.59346 7.72688 5.49994 12 5.49994C16.2731 5.49994 19.0958 9.59346 20.0851 11.2962C20.3496 11.7515 20.3487 12.2996 20.0825 12.7539C19.0908 14.4459 16.2694 18.4999 12 18.4999C7.73064 18.4999 4.90918 14.4459 3.91752 12.7539Z`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,strokeLinejoin:`round`}),(0,l.jsx)(`path`,{d:`M12 14.8261C13.5608 14.8261 14.8261 13.5608 14.8261 12C14.8261 10.4392 13.5608 9.17392 12 9.17392C10.4392 9.17392 9.17391 10.4392 9.17391 12C9.17391 13.5608 10.4392 14.8261 12 14.8261Z`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,strokeLinejoin:`round`})]}),(0,l.jsxs)(`g`,{className:`eye-closed`,style:{opacity:+!t},children:[(0,l.jsx)(`path`,{d:`M18.6025 9.28503C18.9174 8.9701 19.4364 8.99481 19.7015 9.35271C20.1484 9.95606 20.4943 10.507 20.7342 10.9199C21.134 11.6086 21.1329 12.4454 20.7303 13.1328C20.2144 14.013 19.2151 15.5225 17.7723 16.8193C16.3293 18.1162 14.3852 19.2497 12.0008 19.25C11.4192 19.25 10.8638 19.1823 10.3355 19.0613C9.77966 18.934 9.63498 18.2525 10.0382 17.8493C10.2412 17.6463 10.5374 17.573 10.8188 17.6302C11.1993 17.7076 11.5935 17.75 12.0008 17.75C13.8848 17.7497 15.4867 16.8568 16.7693 15.7041C18.0522 14.5511 18.9606 13.1867 19.4363 12.375C19.5656 12.1543 19.5659 11.8943 19.4373 11.6729C19.2235 11.3049 18.921 10.8242 18.5364 10.3003C18.3085 9.98991 18.3302 9.5573 18.6025 9.28503ZM12.0008 4.75C12.5814 4.75006 13.1358 4.81803 13.6632 4.93953C14.2182 5.06741 14.362 5.74812 13.9593 6.15091C13.7558 6.35435 13.4589 6.42748 13.1771 6.36984C12.7983 6.29239 12.4061 6.25006 12.0008 6.25C10.1167 6.25 8.51415 7.15145 7.23028 8.31543C5.94678 9.47919 5.03918 10.8555 4.56426 11.6729C4.43551 11.8945 4.43582 12.1542 4.56524 12.375C4.77587 12.7343 5.07189 13.2012 5.44718 13.7105C5.67623 14.0213 5.65493 14.4552 5.38193 14.7282C5.0671 15.0431 4.54833 15.0189 4.28292 14.6614C3.84652 14.0736 3.50813 13.5369 3.27129 13.1328C2.86831 12.4451 2.86717 11.6088 3.26739 10.9199C3.78185 10.0345 4.77959 8.51239 6.22247 7.2041C7.66547 5.89584 9.61202 4.75 12.0008 4.75Z`,fill:`currentColor`}),(0,l.jsx)(`path`,{d:`M5 19L19 5`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`})]})]}),v=({size:e=24,isPaused:t=!1})=>(0,l.jsxs)(`svg`,{width:e,height:e,viewBox:`0 0 24 24`,fill:`none`,children:[(0,l.jsx)(`style`,{children:`
      .pause-bar, .play-triangle {
        transition: opacity 0.15s ease;
      }
    `}),(0,l.jsx)(`path`,{className:`pause-bar`,d:`M8 6L8 18`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,style:{opacity:+!t}}),(0,l.jsx)(`path`,{className:`pause-bar`,d:`M16 18L16 6`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,style:{opacity:+!t}}),(0,l.jsx)(`path`,{className:`play-triangle`,d:`M17.75 10.701C18.75 11.2783 18.75 12.7217 17.75 13.299L8.75 18.4952C7.75 19.0725 6.5 18.3509 6.5 17.1962L6.5 6.80384C6.5 5.64914 7.75 4.92746 8.75 5.50481L17.75 10.701Z`,stroke:`currentColor`,strokeWidth:`1.5`,style:{opacity:+!!t}})]}),y=({size:e=16})=>(0,l.jsxs)(`svg`,{width:e,height:e,viewBox:`0 0 24 24`,fill:`none`,children:[(0,l.jsx)(`path`,{d:`M10.6504 5.81117C10.9939 4.39628 13.0061 4.39628 13.3496 5.81117C13.5715 6.72517 14.6187 7.15891 15.4219 6.66952C16.6652 5.91193 18.0881 7.33479 17.3305 8.57815C16.8411 9.38134 17.2748 10.4285 18.1888 10.6504C19.6037 10.9939 19.6037 13.0061 18.1888 13.3496C17.2748 13.5715 16.8411 14.6187 17.3305 15.4219C18.0881 16.6652 16.6652 18.0881 15.4219 17.3305C14.6187 16.8411 13.5715 17.2748 13.3496 18.1888C13.0061 19.6037 10.9939 19.6037 10.6504 18.1888C10.4285 17.2748 9.38135 16.8411 8.57815 17.3305C7.33479 18.0881 5.91193 16.6652 6.66952 15.4219C7.15891 14.6187 6.72517 13.5715 5.81117 13.3496C4.39628 13.0061 4.39628 10.9939 5.81117 10.6504C6.72517 10.4285 7.15891 9.38134 6.66952 8.57815C5.91193 7.33479 7.33479 5.91192 8.57815 6.66952C9.38135 7.15891 10.4285 6.72517 10.6504 5.81117Z`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,strokeLinejoin:`round`}),(0,l.jsx)(`circle`,{cx:`12`,cy:`12`,r:`2.5`,stroke:`currentColor`,strokeWidth:`1.5`})]}),re=({size:e=16})=>(0,l.jsx)(`svg`,{width:e,height:e,viewBox:`0 0 24 24`,fill:`none`,children:(0,l.jsx)(`path`,{d:`M13.5 4C14.7426 4 15.75 5.00736 15.75 6.25V7H18.5C18.9142 7 19.25 7.33579 19.25 7.75C19.25 8.16421 18.9142 8.5 18.5 8.5H17.9678L17.6328 16.2217C17.61 16.7475 17.5912 17.1861 17.5469 17.543C17.5015 17.9087 17.4225 18.2506 17.2461 18.5723C16.9747 19.0671 16.5579 19.4671 16.0518 19.7168C15.7227 19.8791 15.3772 19.9422 15.0098 19.9717C14.6514 20.0004 14.2126 20 13.6865 20H10.3135C9.78735 20 9.34856 20.0004 8.99023 19.9717C8.62278 19.9422 8.27729 19.8791 7.94824 19.7168C7.44205 19.4671 7.02532 19.0671 6.75391 18.5723C6.57751 18.2506 6.49853 17.9087 6.45312 17.543C6.40883 17.1861 6.39005 16.7475 6.36719 16.2217L6.03223 8.5H5.5C5.08579 8.5 4.75 8.16421 4.75 7.75C4.75 7.33579 5.08579 7 5.5 7H8.25V6.25C8.25 5.00736 9.25736 4 10.5 4H13.5ZM7.86621 16.1562C7.89013 16.7063 7.90624 17.0751 7.94141 17.3584C7.97545 17.6326 8.02151 17.7644 8.06934 17.8516C8.19271 18.0763 8.38239 18.2577 8.6123 18.3711C8.70153 18.4151 8.83504 18.4545 9.11035 18.4766C9.39482 18.4994 9.76335 18.5 10.3135 18.5H13.6865C14.2367 18.5 14.6052 18.4994 14.8896 18.4766C15.165 18.4545 15.2985 18.4151 15.3877 18.3711C15.6176 18.2577 15.8073 18.0763 15.9307 17.8516C15.9785 17.7644 16.0245 17.6326 16.0586 17.3584C16.0938 17.0751 16.1099 16.7063 16.1338 16.1562L16.4668 8.5H7.5332L7.86621 16.1562ZM9.97656 10.75C10.3906 10.7371 10.7371 11.0626 10.75 11.4766L10.875 15.4766C10.8879 15.8906 10.5624 16.2371 10.1484 16.25C9.73443 16.2629 9.38794 15.9374 9.375 15.5234L9.25 11.5234C9.23706 11.1094 9.56255 10.7629 9.97656 10.75ZM14.0244 10.75C14.4384 10.7635 14.7635 11.1105 14.75 11.5244L14.6201 15.5244C14.6066 15.9384 14.2596 16.2634 13.8457 16.25C13.4317 16.2365 13.1067 15.8896 13.1201 15.4756L13.251 11.4756C13.2645 11.0617 13.6105 10.7366 14.0244 10.75ZM10.5 5.5C10.0858 5.5 9.75 5.83579 9.75 6.25V7H14.25V6.25C14.25 5.83579 13.9142 5.5 13.5 5.5H10.5Z`,fill:`currentColor`})}),ie=({size:e=16})=>(0,l.jsxs)(`svg`,{width:e,height:e,viewBox:`0 0 24 24`,fill:`none`,children:[(0,l.jsxs)(`g`,{clipPath:`url(#clip0_2_53)`,children:[(0,l.jsx)(`path`,{d:`M16.25 16.25L7.75 7.75`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,strokeLinejoin:`round`}),(0,l.jsx)(`path`,{d:`M7.75 16.25L16.25 7.75`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,strokeLinejoin:`round`})]}),(0,l.jsx)(`defs`,{children:(0,l.jsx)(`clipPath`,{id:`clip0_2_53`,children:(0,l.jsx)(`rect`,{width:`24`,height:`24`,fill:`white`})})})]}),ae=({size:e=24})=>(0,l.jsx)(`svg`,{width:e,height:e,viewBox:`0 0 24 24`,fill:`none`,children:(0,l.jsx)(`path`,{d:`M16.7198 6.21973C17.0127 5.92683 17.4874 5.92683 17.7803 6.21973C18.0732 6.51262 18.0732 6.9874 17.7803 7.28027L13.0606 12L17.7803 16.7197C18.0732 17.0126 18.0732 17.4874 17.7803 17.7803C17.4875 18.0731 17.0127 18.0731 16.7198 17.7803L12.0001 13.0605L7.28033 17.7803C6.98746 18.0731 6.51268 18.0731 6.21979 17.7803C5.92689 17.4874 5.92689 17.0126 6.21979 16.7197L10.9395 12L6.21979 7.28027C5.92689 6.98738 5.92689 6.51262 6.21979 6.21973C6.51268 5.92683 6.98744 5.92683 7.28033 6.21973L12.0001 10.9395L16.7198 6.21973Z`,fill:`currentColor`})}),oe=({size:e=16})=>(0,l.jsxs)(`svg`,{width:e,height:e,viewBox:`0 0 24 24`,fill:`none`,children:[(0,l.jsx)(`circle`,{cx:`12`,cy:`12`,r:`4`,stroke:`currentColor`,strokeWidth:`1.5`}),(0,l.jsx)(`path`,{d:`M12 5V3`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`}),(0,l.jsx)(`path`,{d:`M12 21V19`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`}),(0,l.jsx)(`path`,{d:`M16.95 7.05L18.36 5.64`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`}),(0,l.jsx)(`path`,{d:`M5.64 18.36L7.05 16.95`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`}),(0,l.jsx)(`path`,{d:`M19 12H21`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`}),(0,l.jsx)(`path`,{d:`M3 12H5`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`}),(0,l.jsx)(`path`,{d:`M16.95 16.95L18.36 18.36`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`}),(0,l.jsx)(`path`,{d:`M5.64 5.64L7.05 7.05`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`})]}),b=({size:e=16})=>(0,l.jsx)(`svg`,{width:e,height:e,viewBox:`0 0 24 24`,fill:`none`,children:(0,l.jsx)(`path`,{d:`M21 12.79A9 9 0 1111.21 3a7 7 0 009.79 9.79z`,stroke:`currentColor`,strokeWidth:`1.5`,strokeLinecap:`round`,strokeLinejoin:`round`})});function x(e,t=4){let n=[],r=e,i=0;for(;r&&i<t;){let e=r.tagName.toLowerCase();if(e===`html`||e===`body`)break;let t=e;if(r.id)t=`#${r.id}`;else if(r.className&&typeof r.className==`string`){let e=r.className.split(/\s+/).find(e=>e.length>2&&!e.match(/^[a-z]{1,2}$/)&&!e.match(/[A-Z0-9]{5,}/));e&&(t=`.${e.split(`_`)[0]}`)}n.unshift(t),r=r.parentElement,i++}return n.join(` > `)}function S(e){let t=x(e);if(e.dataset.element)return{name:e.dataset.element,path:t};let n=e.tagName.toLowerCase();if([`path`,`circle`,`rect`,`line`,`g`].includes(n)){let n=e.closest(`svg`);if(n){let e=n.parentElement;if(e)return{name:`graphic in ${S(e).name}`,path:t}}return{name:`graphic element`,path:t}}if(n===`svg`){let n=e.parentElement;if(n?.tagName.toLowerCase()===`button`){let e=n.textContent?.trim();return{name:e?`icon in "${e}" button`:`button icon`,path:t}}return{name:`icon`,path:t}}if(n===`button`){let n=e.textContent?.trim(),r=e.getAttribute(`aria-label`);return r?{name:`button [${r}]`,path:t}:{name:n?`button "${n.slice(0,25)}"`:`button`,path:t}}if(n===`a`){let n=e.textContent?.trim(),r=e.getAttribute(`href`);return n?{name:`link "${n.slice(0,25)}"`,path:t}:r?{name:`link to ${r.slice(0,30)}`,path:t}:{name:`link`,path:t}}if(n===`input`){let n=e.getAttribute(`type`)||`text`,r=e.getAttribute(`placeholder`),i=e.getAttribute(`name`);return r?{name:`input "${r}"`,path:t}:i?{name:`input [${i}]`,path:t}:{name:`${n} input`,path:t}}if([`h1`,`h2`,`h3`,`h4`,`h5`,`h6`].includes(n)){let r=e.textContent?.trim();return{name:r?`${n} "${r.slice(0,35)}"`:n,path:t}}if(n===`p`){let n=e.textContent?.trim();return n?{name:`paragraph: "${n.slice(0,40)}${n.length>40?`...`:``}"`,path:t}:{name:`paragraph`,path:t}}if(n===`span`||n===`label`){let r=e.textContent?.trim();return r&&r.length<40?{name:`"${r}"`,path:t}:{name:n,path:t}}if(n===`li`){let n=e.textContent?.trim();return n&&n.length<40?{name:`list item: "${n.slice(0,35)}"`,path:t}:{name:`list item`,path:t}}if(n===`blockquote`)return{name:`blockquote`,path:t};if(n===`code`){let n=e.textContent?.trim();return n&&n.length<30?{name:`code: \`${n}\``,path:t}:{name:`code`,path:t}}if(n===`pre`)return{name:`code block`,path:t};if(n===`img`){let n=e.getAttribute(`alt`);return{name:n?`image "${n.slice(0,30)}"`:`image`,path:t}}if(n===`video`)return{name:`video`,path:t};if([`div`,`section`,`article`,`nav`,`header`,`footer`,`aside`,`main`].includes(n)){let r=e.className,i=e.getAttribute(`role`),a=e.getAttribute(`aria-label`);if(a)return{name:`${n} [${a}]`,path:t};if(i)return{name:`${i}`,path:t};if(typeof r==`string`&&r){let e=r.split(/[\s_-]+/).map(e=>e.replace(/[A-Z0-9]{5,}.*$/,``)).filter(e=>e.length>2&&!/^[a-z]{1,2}$/.test(e)).slice(0,2);if(e.length>0)return{name:e.join(` `),path:t}}return{name:n===`div`?`container`:n,path:t}}return{name:n,path:t}}function C(e){let t=[],n=e.textContent?.trim();n&&n.length<100&&t.push(n);let r=e.previousElementSibling;if(r){let e=r.textContent?.trim();e&&e.length<50&&t.unshift(`[before: "${e.slice(0,40)}"]`)}let i=e.nextElementSibling;if(i){let e=i.textContent?.trim();e&&e.length<50&&t.push(`[after: "${e.slice(0,40)}"]`)}return t.join(` `)}function w(e){let t=e.parentElement;if(!t)return``;let n=Array.from(t.children).filter(t=>t!==e&&t instanceof HTMLElement);if(n.length===0)return``;let r=n.slice(0,4).map(e=>{let t=e.tagName.toLowerCase(),n=e.className,r=``;if(typeof n==`string`&&n){let e=n.split(/\s+/).map(e=>e.replace(/[_][a-zA-Z0-9]{5,}.*$/,``)).find(e=>e.length>2&&!/^[a-z]{1,2}$/.test(e));e&&(r=`.${e}`)}if(t===`button`||t===`a`){let n=e.textContent?.trim().slice(0,15);if(n)return`${t}${r} "${n}"`}return`${t}${r}`}),i=t.tagName.toLowerCase();if(typeof t.className==`string`&&t.className){let e=t.className.split(/\s+/).map(e=>e.replace(/[_][a-zA-Z0-9]{5,}.*$/,``)).find(e=>e.length>2&&!/^[a-z]{1,2}$/.test(e));e&&(i=`.${e}`)}let a=t.children.length,o=a>r.length+1?` (${a} total in ${i})`:``;return r.join(`, `)+o}function se(e){let t=e.className;return typeof t!=`string`||!t?``:t.split(/\s+/).filter(e=>e.length>0).map(e=>{let t=e.match(/^([a-zA-Z][a-zA-Z0-9_-]*?)(?:_[a-zA-Z0-9]{5,})?$/);return t?t[1]:e}).filter((e,t,n)=>n.indexOf(e)===t).join(`, `)}var T=new Set([`none`,`normal`,`auto`,`0px`,`rgba(0, 0, 0, 0)`,`transparent`,`static`,`visible`]),E=new Set(`p.span.h1.h2.h3.h4.h5.h6.label.li.td.th.blockquote.figcaption.caption.legend.dt.dd.pre.code.em.strong.b.i.a.time.cite.q`.split(`.`)),D=new Set([`input`,`textarea`,`select`]),ce=new Set([`img`,`video`,`canvas`,`svg`]),O=new Set([`div`,`section`,`article`,`nav`,`header`,`footer`,`aside`,`main`,`ul`,`ol`,`form`,`fieldset`]);function le(e){if(typeof window>`u`)return{};let t=window.getComputedStyle(e),n={},r=e.tagName.toLowerCase(),i;i=E.has(r)?[`color`,`fontSize`,`fontWeight`,`fontFamily`,`lineHeight`]:r===`button`||r===`a`&&e.getAttribute(`role`)===`button`||D.has(r)?[`backgroundColor`,`color`,`padding`,`borderRadius`,`fontSize`]:ce.has(r)?[`width`,`height`,`objectFit`,`borderRadius`]:O.has(r)?[`display`,`padding`,`margin`,`gap`,`backgroundColor`]:[`color`,`fontSize`,`margin`,`padding`,`backgroundColor`];for(let e of i){let r=e.replace(/([A-Z])/g,`-$1`).toLowerCase(),i=t.getPropertyValue(r);i&&!T.has(i)&&(n[e]=i)}return n}var ue=`color.backgroundColor.borderColor.fontSize.fontWeight.fontFamily.lineHeight.letterSpacing.textAlign.width.height.padding.margin.border.borderRadius.display.position.top.right.bottom.left.zIndex.flexDirection.justifyContent.alignItems.gap.opacity.visibility.overflow.boxShadow.transform`.split(`.`);function de(e){if(typeof window>`u`)return``;let t=window.getComputedStyle(e),n=[];for(let e of ue){let r=e.replace(/([A-Z])/g,`-$1`).toLowerCase(),i=t.getPropertyValue(r);i&&!T.has(i)&&n.push(`${r}: ${i}`)}return n.join(`; `)}function fe(e){if(!e)return;let t={},n=e.split(`;`).map(e=>e.trim()).filter(Boolean);for(let e of n){let n=e.indexOf(`:`);if(n>0){let r=e.slice(0,n).trim(),i=e.slice(n+1).trim();r&&i&&(t[r]=i)}}return Object.keys(t).length>0?t:void 0}function pe(e){let t=[],n=e.getAttribute(`role`),r=e.getAttribute(`aria-label`),i=e.getAttribute(`aria-describedby`),a=e.getAttribute(`tabindex`),o=e.getAttribute(`aria-hidden`);return n&&t.push(`role="${n}"`),r&&t.push(`aria-label="${r}"`),i&&t.push(`aria-describedby="${i}"`),a&&t.push(`tabindex=${a}`),o===`true`&&t.push(`aria-hidden`),e.matches(`a, button, input, select, textarea, [tabindex]`)&&t.push(`focusable`),t.join(`, `)}function me(e){let t=[],n=e;for(;n&&n.tagName.toLowerCase()!==`html`;){let e=n.tagName.toLowerCase(),r=e;if(n.id)r=`${e}#${n.id}`;else if(n.className&&typeof n.className==`string`){let t=n.className.split(/\s+/).map(e=>e.replace(/[_][a-zA-Z0-9]{5,}.*$/,``)).find(e=>e.length>2);t&&(r=`${e}.${t}`)}t.unshift(r),n=n.parentElement}return t.join(` > `)}var k=`feedback-annotations-`,he=7;function ge(e){return`${k}${e}`}function _e(e){if(typeof window>`u`)return[];try{let t=localStorage.getItem(ge(e));if(!t)return[];let n=JSON.parse(t),r=Date.now()-he*24*60*60*1e3;return n.filter(e=>!e.timestamp||e.timestamp>r)}catch{return[]}}function ve(e,t){if(!(typeof window>`u`))try{localStorage.setItem(ge(e),JSON.stringify(t))}catch{}}var A=`@keyframes styles-module__toolbarEnter___u8RRu {
  from {
    opacity: 0;
    transform: scale(0.5) rotate(90deg);
  }
  to {
    opacity: 1;
    transform: scale(1) rotate(0deg);
  }
}
@keyframes styles-module__badgeEnter___mVQLj {
  from {
    opacity: 0;
    transform: scale(0);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
@keyframes styles-module__scaleIn___c-r1K {
  from {
    opacity: 0;
    transform: scale(0.85);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
@keyframes styles-module__scaleOut___Wctwz {
  from {
    opacity: 1;
    transform: scale(1);
  }
  to {
    opacity: 0;
    transform: scale(0.85);
  }
}
@keyframes styles-module__slideUp___kgD36 {
  from {
    opacity: 0;
    transform: scale(0.85) translateY(8px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}
@keyframes styles-module__slideDown___zcdje {
  from {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
  to {
    opacity: 0;
    transform: scale(0.85) translateY(8px);
  }
}
@keyframes styles-module__markerIn___5FaAP {
  0% {
    opacity: 0;
    transform: translate(-50%, -50%) scale(0.3);
  }
  100% {
    opacity: 1;
    transform: translate(-50%, -50%) scale(1);
  }
}
@keyframes styles-module__markerOut___GU5jX {
  0% {
    opacity: 1;
    transform: translate(-50%, -50%) scale(1);
  }
  100% {
    opacity: 0;
    transform: translate(-50%, -50%) scale(0.3);
  }
}
@keyframes styles-module__fadeIn___b9qmf {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}
@keyframes styles-module__fadeOut___6Ut6- {
  from {
    opacity: 1;
  }
  to {
    opacity: 0;
  }
}
@keyframes styles-module__tooltipIn___0N31w {
  from {
    opacity: 0;
    transform: translateX(-50%) translateY(2px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateX(-50%) translateY(0) scale(1);
  }
}
@keyframes styles-module__hoverHighlightIn___6WYHY {
  from {
    opacity: 0;
    transform: scale(0.98);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
@keyframes styles-module__hoverTooltipIn___FYGQx {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(4px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}
@keyframes styles-module__settingsPanelIn___MGfO8 {
  from {
    opacity: 0;
    transform: translateY(10px) scale(0.95);
    filter: blur(5px);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
    filter: blur(0px);
  }
}
@keyframes styles-module__settingsPanelOut___Zfymi {
  from {
    opacity: 1;
    transform: translateY(0) scale(1);
    filter: blur(0px);
  }
  to {
    opacity: 0;
    transform: translateY(20px) scale(0.95);
    filter: blur(5px);
  }
}
.styles-module__toolbar___wNsdK {
  position: fixed;
  bottom: 1.25rem;
  right: 1.25rem;
  width: 257px;
  z-index: 100000;
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  pointer-events: none;
  transition: left 0s, top 0s, right 0s, bottom 0s;
}

.styles-module__toolbarContainer___dIhma {
  user-select: none;
  margin-left: auto;
  align-self: flex-end;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #1a1a1a;
  color: #fff;
  border: none;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2), 0 4px 16px rgba(0, 0, 0, 0.1);
  pointer-events: auto;
  cursor: grab;
  transition: width 0.4s cubic-bezier(0.19, 1, 0.22, 1), transform 0.4s cubic-bezier(0.19, 1, 0.22, 1);
}
.styles-module__toolbarContainer___dIhma.styles-module__dragging___xrolZ {
  transition: width 0.4s cubic-bezier(0.19, 1, 0.22, 1);
  cursor: grabbing;
}
.styles-module__toolbarContainer___dIhma.styles-module__entrance___sgHd8 {
  animation: styles-module__toolbarEnter___u8RRu 0.5s cubic-bezier(0.34, 1.2, 0.64, 1) forwards;
}
.styles-module__toolbarContainer___dIhma.styles-module__collapsed___Rydsn {
  width: 44px;
  height: 44px;
  border-radius: 22px;
  padding: 0;
  cursor: pointer;
}
.styles-module__toolbarContainer___dIhma.styles-module__collapsed___Rydsn svg {
  margin-top: -1px;
}
.styles-module__toolbarContainer___dIhma.styles-module__collapsed___Rydsn:hover {
  background: #2a2a2a;
}
.styles-module__toolbarContainer___dIhma.styles-module__collapsed___Rydsn:active {
  transform: scale(0.95);
}
.styles-module__toolbarContainer___dIhma.styles-module__expanded___ofKPx {
  width: calc-size(auto, size);
  height: 44px;
  border-radius: 1.5rem;
  padding: 0.375rem;
}
@supports not (width: calc-size(auto, size)) {
  .styles-module__toolbarContainer___dIhma.styles-module__expanded___ofKPx {
    width: 257px;
  }
}

.styles-module__toggleContent___0yfyP {
  position: absolute;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: opacity 0.1s cubic-bezier(0.19, 1, 0.22, 1);
}
.styles-module__toggleContent___0yfyP.styles-module__visible___KHwEW {
  opacity: 1;
  visibility: visible;
  pointer-events: auto;
}
.styles-module__toggleContent___0yfyP.styles-module__hidden___Ae8H4 {
  opacity: 0;
  pointer-events: none;
}

.styles-module__controlsContent___9GJWU {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  transition: filter 0.8s cubic-bezier(0.19, 1, 0.22, 1), opacity 0.8s cubic-bezier(0.19, 1, 0.22, 1), transform 0.6s cubic-bezier(0.19, 1, 0.22, 1);
}
.styles-module__controlsContent___9GJWU.styles-module__visible___KHwEW {
  opacity: 1;
  filter: blur(0px);
  transform: scale(1);
  visibility: visible;
  pointer-events: auto;
}
.styles-module__controlsContent___9GJWU.styles-module__hidden___Ae8H4 {
  opacity: 0;
  filter: blur(10px);
  transform: scale(0.4);
  pointer-events: none;
}

.styles-module__badge___2XsgF {
  position: absolute;
  top: -16px;
  right: -16px;
  user-select: none;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: 9px;
  background: #3c82f7;
  color: white;
  font-size: 0.625rem;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
  opacity: 1;
  transition: transform 0.3s ease, opacity 0.2s ease;
  transform: scale(1);
}
.styles-module__badge___2XsgF.styles-module__fadeOut___6Ut6- {
  opacity: 0;
  transform: scale(0);
  pointer-events: none;
}
.styles-module__badge___2XsgF.styles-module__entrance___sgHd8 {
  animation: styles-module__badgeEnter___mVQLj 0.3s cubic-bezier(0.34, 1.2, 0.64, 1) 0.4s both;
}

.styles-module__controlButton___8Q0jc {
  position: relative;
  cursor: pointer !important;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border-radius: 50%;
  border: none;
  background: transparent;
  color: rgba(255, 255, 255, 0.85);
  transition: background-color 0.15s ease, color 0.15s ease, transform 0.1s ease, opacity 0.2s ease;
}
.styles-module__controlButton___8Q0jc:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
}
.styles-module__controlButton___8Q0jc:active:not(:disabled) {
  transform: scale(0.92);
}
.styles-module__controlButton___8Q0jc:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
.styles-module__controlButton___8Q0jc[data-active=true] {
  color: #3c82f7;
  background: rgba(60, 130, 247, 0.25);
}
.styles-module__controlButton___8Q0jc[data-danger]:hover:not(:disabled) {
  background: rgba(255, 59, 48, 0.25);
  color: #ff3b30;
}

.styles-module__buttonWrapper___rBcdv {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}
.styles-module__buttonWrapper___rBcdv:hover .styles-module__buttonTooltip___Burd9 {
  opacity: 1;
  visibility: visible;
  transform: translateX(-50%) scale(1);
  transition-delay: 0.85s;
}
.styles-module__buttonWrapper___rBcdv:has(.styles-module__controlButton___8Q0jc:disabled):hover .styles-module__buttonTooltip___Burd9 {
  opacity: 0;
  visibility: hidden;
}

.styles-module__buttonTooltip___Burd9 {
  position: absolute;
  bottom: calc(100% + 14px);
  left: 50%;
  transform: translateX(-50%) scale(0.95);
  padding: 6px 10px;
  background: #1a1a1a;
  color: rgba(255, 255, 255, 0.9);
  font-size: 12px;
  font-weight: 500;
  border-radius: 8px;
  white-space: nowrap;
  opacity: 0;
  visibility: hidden;
  pointer-events: none;
  z-index: 100001;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  transition: opacity 0.135s ease, transform 0.135s ease, visibility 0.135s ease;
}
.styles-module__buttonTooltip___Burd9::after {
  content: "";
  position: absolute;
  top: calc(100% - 4px);
  left: 50%;
  transform: translateX(-50%) rotate(45deg);
  width: 8px;
  height: 8px;
  background: #1a1a1a;
  border-radius: 0 0 2px 0;
}

.styles-module__shortcut___lEAQk {
  margin-left: 4px;
  opacity: 0.5;
}

.styles-module__tooltipBelow___m6ats .styles-module__buttonTooltip___Burd9 {
  bottom: auto;
  top: calc(100% + 14px);
  transform: translateX(-50%) scale(0.95);
}
.styles-module__tooltipBelow___m6ats .styles-module__buttonTooltip___Burd9::after {
  top: -4px;
  bottom: auto;
  border-radius: 2px 0 0 0;
}

.styles-module__tooltipBelow___m6ats .styles-module__buttonWrapper___rBcdv:hover .styles-module__buttonTooltip___Burd9 {
  transform: translateX(-50%) scale(1);
}

.styles-module__tooltipsHidden___VtLJG .styles-module__buttonTooltip___Burd9 {
  opacity: 0 !important;
  visibility: hidden !important;
  transition: none !important;
}

.styles-module__buttonWrapperAlignLeft___myzIp .styles-module__buttonTooltip___Burd9 {
  left: 50%;
  transform: translateX(-12px) scale(0.95);
}
.styles-module__buttonWrapperAlignLeft___myzIp .styles-module__buttonTooltip___Burd9::after {
  left: 16px;
}
.styles-module__buttonWrapperAlignLeft___myzIp:hover .styles-module__buttonTooltip___Burd9 {
  transform: translateX(-12px) scale(1);
}

.styles-module__tooltipBelow___m6ats .styles-module__buttonWrapperAlignLeft___myzIp .styles-module__buttonTooltip___Burd9 {
  transform: translateX(-12px) scale(0.95);
}
.styles-module__tooltipBelow___m6ats .styles-module__buttonWrapperAlignLeft___myzIp:hover .styles-module__buttonTooltip___Burd9 {
  transform: translateX(-12px) scale(1);
}

.styles-module__buttonWrapperAlignRight___HCQFR .styles-module__buttonTooltip___Burd9 {
  left: 50%;
  transform: translateX(calc(-100% + 12px)) scale(0.95);
}
.styles-module__buttonWrapperAlignRight___HCQFR .styles-module__buttonTooltip___Burd9::after {
  left: auto;
  right: 8px;
}
.styles-module__buttonWrapperAlignRight___HCQFR:hover .styles-module__buttonTooltip___Burd9 {
  transform: translateX(calc(-100% + 12px)) scale(1);
}

.styles-module__tooltipBelow___m6ats .styles-module__buttonWrapperAlignRight___HCQFR .styles-module__buttonTooltip___Burd9 {
  transform: translateX(calc(-100% + 12px)) scale(0.95);
}
.styles-module__tooltipBelow___m6ats .styles-module__buttonWrapperAlignRight___HCQFR:hover .styles-module__buttonTooltip___Burd9 {
  transform: translateX(calc(-100% + 12px)) scale(1);
}

.styles-module__divider___c--s1 {
  width: 1px;
  height: 12px;
  background: rgba(255, 255, 255, 0.15);
  margin: 0 0.125rem;
}

.styles-module__overlay___Q1O9y {
  position: fixed;
  inset: 0;
  z-index: 99997;
  pointer-events: none;
}
.styles-module__overlay___Q1O9y > * {
  pointer-events: auto;
}

.styles-module__hoverHighlight___ogakW {
  position: fixed;
  border: 2px solid rgba(60, 130, 247, 0.5);
  border-radius: 4px;
  pointer-events: none !important;
  background: rgba(60, 130, 247, 0.04);
  box-sizing: border-box;
  will-change: opacity;
  contain: layout style;
}
.styles-module__hoverHighlight___ogakW.styles-module__enter___WFIki {
  animation: styles-module__hoverHighlightIn___6WYHY 0.12s ease-out forwards;
}

.styles-module__multiSelectOutline___cSJ-m {
  position: fixed;
  border: 2px dashed rgba(52, 199, 89, 0.6);
  border-radius: 4px;
  pointer-events: none !important;
  background: rgba(52, 199, 89, 0.05);
  box-sizing: border-box;
  will-change: opacity;
}
.styles-module__multiSelectOutline___cSJ-m.styles-module__enter___WFIki {
  animation: styles-module__fadeIn___b9qmf 0.15s ease-out forwards;
}
.styles-module__multiSelectOutline___cSJ-m.styles-module__exit___fyOJ0 {
  animation: styles-module__fadeOut___6Ut6- 0.15s ease-out forwards;
}

.styles-module__singleSelectOutline___QhX-O {
  position: fixed;
  border: 2px solid rgba(60, 130, 247, 0.6);
  border-radius: 4px;
  pointer-events: none !important;
  background: rgba(60, 130, 247, 0.05);
  box-sizing: border-box;
  will-change: opacity;
}
.styles-module__singleSelectOutline___QhX-O.styles-module__enter___WFIki {
  animation: styles-module__fadeIn___b9qmf 0.15s ease-out forwards;
}
.styles-module__singleSelectOutline___QhX-O.styles-module__exit___fyOJ0 {
  animation: styles-module__fadeOut___6Ut6- 0.15s ease-out forwards;
}

.styles-module__hoverTooltip___bvLk7 {
  position: fixed;
  font-size: 0.6875rem;
  font-weight: 500;
  color: #fff;
  background: rgba(0, 0, 0, 0.85);
  padding: 0.35rem 0.6rem;
  border-radius: 0.375rem;
  pointer-events: none !important;
  white-space: nowrap;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.styles-module__hoverTooltip___bvLk7.styles-module__enter___WFIki {
  animation: styles-module__hoverTooltipIn___FYGQx 0.1s ease-out forwards;
}

.styles-module__markersLayer___-25j1 {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 0;
  z-index: 99998;
  pointer-events: none;
}
.styles-module__markersLayer___-25j1 > * {
  pointer-events: auto;
}

.styles-module__fixedMarkersLayer___ffyX6 {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 99998;
  pointer-events: none;
}
.styles-module__fixedMarkersLayer___ffyX6 > * {
  pointer-events: auto;
}

.styles-module__marker___6sQrs {
  position: absolute;
  width: 22px;
  height: 22px;
  background: #3c82f7;
  color: white;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.6875rem;
  font-weight: 600;
  transform: translate(-50%, -50%) scale(1);
  opacity: 1;
  cursor: pointer;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.2), inset 0 0 0 1px rgba(0, 0, 0, 0.04);
  user-select: none;
  will-change: transform, opacity;
  contain: layout style;
  z-index: 1;
}
.styles-module__marker___6sQrs:hover {
  z-index: 2;
}
.styles-module__marker___6sQrs:not(.styles-module__enter___WFIki):not(.styles-module__exit___fyOJ0):not(.styles-module__clearing___FQ--7) {
  transition: background-color 0.15s ease, transform 0.1s ease;
}
.styles-module__marker___6sQrs.styles-module__enter___WFIki {
  animation: styles-module__markerIn___5FaAP 0.25s cubic-bezier(0.22, 1, 0.36, 1) both;
}
.styles-module__marker___6sQrs.styles-module__exit___fyOJ0 {
  animation: styles-module__markerOut___GU5jX 0.2s ease-out both;
  pointer-events: none;
}
.styles-module__marker___6sQrs.styles-module__clearing___FQ--7 {
  animation: styles-module__markerOut___GU5jX 0.15s ease-out both;
  pointer-events: none;
}
.styles-module__marker___6sQrs:not(.styles-module__enter___WFIki):not(.styles-module__exit___fyOJ0):not(.styles-module__clearing___FQ--7):hover {
  transform: translate(-50%, -50%) scale(1.1);
}
.styles-module__marker___6sQrs.styles-module__pending___2IHLC {
  position: fixed;
  background: #3c82f7;
}
.styles-module__marker___6sQrs.styles-module__fixed___dBMHC {
  position: fixed;
}
.styles-module__marker___6sQrs.styles-module__multiSelect___YWiuz {
  background: #34c759;
  width: 26px;
  height: 26px;
  border-radius: 6px;
  font-size: 0.75rem;
}
.styles-module__marker___6sQrs.styles-module__multiSelect___YWiuz.styles-module__pending___2IHLC {
  background: #34c759;
}
.styles-module__marker___6sQrs.styles-module__hovered___ZgXIy {
  background: #ff3b30;
}

.styles-module__renumber___nCTxD {
  display: block;
  animation: styles-module__renumberRoll___Wgbq3 0.2s ease-out;
}

@keyframes styles-module__renumberRoll___Wgbq3 {
  0% {
    transform: translateX(-40%);
    opacity: 0;
  }
  100% {
    transform: translateX(0);
    opacity: 1;
  }
}
.styles-module__markerTooltip___aLJID {
  position: absolute;
  top: calc(100% + 10px);
  left: 50%;
  transform: translateX(-50%);
  z-index: 100002;
  background: #1a1a1a;
  padding: 0.625rem 0.75rem;
  border-radius: 0.75rem;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3), 0 0 0 1px rgba(255, 255, 255, 0.08);
  min-width: 120px;
  max-width: 200px;
  pointer-events: none;
  cursor: default;
}
.styles-module__markerTooltip___aLJID.styles-module__enter___WFIki {
  animation: styles-module__tooltipIn___0N31w 0.1s ease-out forwards;
}

.styles-module__markerQuote___FHmrz {
  display: block;
  font-size: 0.6875rem;
  font-style: italic;
  color: rgba(255, 255, 255, 0.5);
  margin-bottom: 0.375rem;
  line-height: 1.4;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.styles-module__markerNote___QkrrS {
  display: block;
  font-size: 0.75rem;
  font-weight: 450;
  line-height: 1.4;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  padding-bottom: 2px;
}

.styles-module__markerHint___2iF-6 {
  display: block;
  font-size: 0.625rem;
  font-weight: 400;
  color: rgba(255, 255, 255, 0.3);
  margin-top: 0.375rem;
  white-space: nowrap;
}

.styles-module__settingsPanel___OxX3Y {
  position: absolute;
  right: 5px;
  bottom: calc(100% + 0.5rem);
  z-index: 1;
  background: white;
  border-radius: 1rem;
  padding: 13px 1rem 16px;
  min-width: 205px;
  box-shadow: 0 1px 8px rgba(0, 0, 0, 0.25), 0 0 0 1px rgba(0, 0, 0, 0.04);
  transition: background 0.25s ease, box-shadow 0.25s ease;
}
.styles-module__settingsPanel___OxX3Y .styles-module__settingsHeader___pwDY9,
.styles-module__settingsPanel___OxX3Y .styles-module__settingsBrand___0gJeM,
.styles-module__settingsPanel___OxX3Y .styles-module__settingsBrandSlash___uTG18,
.styles-module__settingsPanel___OxX3Y .styles-module__settingsVersion___TUcFq,
.styles-module__settingsPanel___OxX3Y .styles-module__settingsSection___m-YM2,
.styles-module__settingsPanel___OxX3Y .styles-module__settingsLabel___8UjfX,
.styles-module__settingsPanel___OxX3Y .styles-module__cycleButton___FMKfw,
.styles-module__settingsPanel___OxX3Y .styles-module__cycleDot___nPgLY,
.styles-module__settingsPanel___OxX3Y .styles-module__dropdownButton___16NPz,
.styles-module__settingsPanel___OxX3Y .styles-module__toggleLabel___Xm8Aa,
.styles-module__settingsPanel___OxX3Y .styles-module__customCheckbox___U39ax,
.styles-module__settingsPanel___OxX3Y .styles-module__sliderLabel___U8sPr,
.styles-module__settingsPanel___OxX3Y .styles-module__slider___GLdxp,
.styles-module__settingsPanel___OxX3Y .styles-module__helpIcon___xQg56,
.styles-module__settingsPanel___OxX3Y .styles-module__themeToggle___2rUjA {
  transition: background 0.25s ease, color 0.25s ease, border-color 0.25s ease;
}
.styles-module__settingsPanel___OxX3Y.styles-module__enter___WFIki {
  opacity: 1;
  transform: translateY(0) scale(1);
  filter: blur(0px);
  transition: opacity 0.2s ease, transform 0.2s ease, filter 0.2s ease;
}
.styles-module__settingsPanel___OxX3Y.styles-module__exit___fyOJ0 {
  opacity: 0;
  transform: translateY(8px) scale(0.95);
  filter: blur(5px);
  pointer-events: none;
  transition: opacity 0.1s ease, transform 0.1s ease, filter 0.1s ease;
}
.styles-module__settingsPanel___OxX3Y.styles-module__dark___ILIQf {
  background: #1a1a1a;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3), 0 0 0 1px rgba(255, 255, 255, 0.08);
}
.styles-module__settingsPanel___OxX3Y.styles-module__dark___ILIQf .styles-module__settingsLabel___8UjfX {
  color: rgba(255, 255, 255, 0.6);
}
.styles-module__settingsPanel___OxX3Y.styles-module__dark___ILIQf .styles-module__settingsOption___UNa12 {
  color: rgba(255, 255, 255, 0.85);
}
.styles-module__settingsPanel___OxX3Y.styles-module__dark___ILIQf .styles-module__settingsOption___UNa12:hover {
  background: rgba(255, 255, 255, 0.1);
}
.styles-module__settingsPanel___OxX3Y.styles-module__dark___ILIQf .styles-module__settingsOption___UNa12.styles-module__selected___OwRqP {
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
}
.styles-module__settingsPanel___OxX3Y.styles-module__dark___ILIQf .styles-module__toggleLabel___Xm8Aa {
  color: rgba(255, 255, 255, 0.85);
}

.styles-module__settingsHeader___pwDY9 {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 24px;
  margin-bottom: 0.5rem;
  padding-bottom: 9px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.07);
}

.styles-module__settingsBrand___0gJeM {
  font-size: 0.8125rem;
  font-weight: 600;
  letter-spacing: -0.0094em;
  color: #fff;
}

.styles-module__settingsBrandSlash___uTG18 {
  color: rgba(255, 255, 255, 0.5);
}

.styles-module__settingsVersion___TUcFq {
  font-size: 0.6875rem;
  font-weight: 400;
  color: rgba(255, 255, 255, 0.4);
  margin-left: auto;
  letter-spacing: -0.0094em;
}

.styles-module__settingsSection___m-YM2 + .styles-module__settingsSection___m-YM2 {
  margin-top: 0.5rem;
  padding-top: calc(0.5rem + 2px);
  border-top: 1px solid rgba(255, 255, 255, 0.07);
}

.styles-module__settingsRow___3sdhc {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 24px;
}

.styles-module__dropdownContainer___BVnxe {
  position: relative;
}

.styles-module__dropdownButton___16NPz {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.25rem 0.5rem;
  border: none;
  border-radius: 0.375rem;
  background: transparent;
  font-size: 0.8125rem;
  font-weight: 600;
  color: #fff;
  cursor: pointer;
  transition: background-color 0.15s ease, color 0.15s ease;
  letter-spacing: -0.0094em;
}
.styles-module__dropdownButton___16NPz:hover {
  background: rgba(255, 255, 255, 0.08);
}
.styles-module__dropdownButton___16NPz svg {
  opacity: 0.6;
}

.styles-module__cycleButton___FMKfw {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0;
  border: none;
  background: transparent;
  font-size: 0.8125rem;
  font-weight: 500;
  color: #fff;
  cursor: pointer;
  letter-spacing: -0.0094em;
}
.styles-module__cycleButton___FMKfw.styles-module__light___r6n4Y {
  color: rgba(0, 0, 0, 0.85);
}

@keyframes styles-module__cycleTextIn___Q6zJf {
  0% {
    opacity: 0;
    transform: translateY(-6px);
  }
  100% {
    opacity: 1;
    transform: translateY(0);
  }
}
.styles-module__cycleButtonText___fD1LR {
  display: inline-block;
  animation: styles-module__cycleTextIn___Q6zJf 0.2s ease-out;
}

.styles-module__cycleDots___LWuoQ {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.styles-module__cycleDot___nPgLY {
  width: 3px;
  height: 3px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.3);
  transform: scale(0.667);
  transition: background-color 0.25s ease-out, transform 0.25s ease-out;
}
.styles-module__cycleDot___nPgLY.styles-module__active___-zoN6 {
  background: #fff;
  transform: scale(1);
}
.styles-module__cycleDot___nPgLY.styles-module__light___r6n4Y {
  background: rgba(0, 0, 0, 0.2);
}
.styles-module__cycleDot___nPgLY.styles-module__light___r6n4Y.styles-module__active___-zoN6 {
  background: rgba(0, 0, 0, 0.7);
}

.styles-module__dropdownMenu___k73ER {
  position: absolute;
  right: 0;
  top: calc(100% + 0.25rem);
  background: #1a1a1a;
  border-radius: 0.5rem;
  padding: 0.25rem;
  min-width: 120px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3), 0 0 0 1px rgba(255, 255, 255, 0.1);
  z-index: 10;
  animation: styles-module__scaleIn___c-r1K 0.15s ease-out;
}

.styles-module__dropdownItem___ylsLj {
  width: 100%;
  display: flex;
  align-items: center;
  padding: 0.5rem 0.625rem;
  border: none;
  border-radius: 0.375rem;
  background: transparent;
  font-size: 0.8125rem;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.85);
  cursor: pointer;
  text-align: left;
  transition: background-color 0.15s ease, color 0.15s ease;
  letter-spacing: -0.0094em;
}
.styles-module__dropdownItem___ylsLj:hover {
  background: rgba(255, 255, 255, 0.08);
}
.styles-module__dropdownItem___ylsLj.styles-module__selected___OwRqP {
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
  font-weight: 600;
}

.styles-module__settingsLabel___8UjfX {
  font-size: 0.8125rem;
  font-weight: 400;
  letter-spacing: -0.0094em;
  color: rgba(255, 255, 255, 0.5);
  display: flex;
  align-items: center;
  gap: 0.125rem;
}
.styles-module__settingsLabel___8UjfX.styles-module__light___r6n4Y {
  color: rgba(0, 0, 0, 0.5);
}

.styles-module__settingsLabelMarker___ewdtV {
  margin-bottom: 10px;
}

.styles-module__settingsOptions___LyrBA {
  display: flex;
  gap: 0.25rem;
}

.styles-module__settingsOption___UNa12 {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.25rem;
  padding: 0.375rem 0.5rem;
  border: none;
  border-radius: 0.375rem;
  background: transparent;
  font-size: 0.6875rem;
  font-weight: 500;
  color: rgba(0, 0, 0, 0.7);
  cursor: pointer;
  transition: background-color 0.15s ease, color 0.15s ease;
}
.styles-module__settingsOption___UNa12:hover {
  background: rgba(0, 0, 0, 0.05);
}
.styles-module__settingsOption___UNa12.styles-module__selected___OwRqP {
  background: rgba(60, 130, 247, 0.15);
  color: #3c82f7;
}

.styles-module__sliderContainer___ducXj {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.styles-module__slider___GLdxp {
  -webkit-appearance: none;
  appearance: none;
  width: 100%;
  height: 4px;
  background: rgba(255, 255, 255, 0.15);
  border-radius: 2px;
  outline: none;
  cursor: pointer;
}
.styles-module__slider___GLdxp::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 14px;
  height: 14px;
  background: white;
  border-radius: 50%;
  cursor: pointer;
  transition: transform 0.15s ease, box-shadow 0.15s ease;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}
.styles-module__slider___GLdxp::-moz-range-thumb {
  width: 14px;
  height: 14px;
  background: white;
  border: none;
  border-radius: 50%;
  cursor: pointer;
  transition: transform 0.15s ease, box-shadow 0.15s ease;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}
.styles-module__slider___GLdxp:hover::-webkit-slider-thumb {
  transform: scale(1.15);
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4);
}
.styles-module__slider___GLdxp:hover::-moz-range-thumb {
  transform: scale(1.15);
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4);
}

.styles-module__sliderLabels___FhLDB {
  display: flex;
  justify-content: space-between;
}

.styles-module__sliderLabel___U8sPr {
  font-size: 0.625rem;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.4);
  cursor: pointer;
  transition: color 0.15s ease;
}
.styles-module__sliderLabel___U8sPr:hover {
  color: rgba(255, 255, 255, 0.7);
}
.styles-module__sliderLabel___U8sPr.styles-module__active___-zoN6 {
  color: rgba(255, 255, 255, 0.9);
}

.styles-module__colorOptions___iHCNX {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.375rem;
  margin-bottom: 1px;
}

.styles-module__colorOption___IodiY {
  display: block;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  transition: transform 0.2s cubic-bezier(0.25, 1, 0.5, 1);
}
.styles-module__colorOption___IodiY:hover {
  transform: scale(1.15);
}
.styles-module__colorOption___IodiY.styles-module__selected___OwRqP {
  transform: scale(0.83);
}

.styles-module__colorOptionRing___U2xpo {
  display: flex;
  width: 24px;
  height: 24px;
  border: 2px solid transparent;
  border-radius: 50%;
  transition: border-color 0.3s ease;
}
.styles-module__settingsToggle___fBrFn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}
.styles-module__settingsToggle___fBrFn + .styles-module__settingsToggle___fBrFn {
  margin-top: calc(0.5rem + 6px);
}
.styles-module__settingsToggle___fBrFn input[type=checkbox] {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}

.styles-module__customCheckbox___U39ax {
  position: relative;
  width: 14px;
  height: 14px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.05);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background 0.25s ease, border-color 0.25s ease;
}
.styles-module__customCheckbox___U39ax svg {
  color: #1a1a1a;
  opacity: 1;
  transition: opacity 0.15s ease;
}
input[type=checkbox]:checked + .styles-module__customCheckbox___U39ax {
  border-color: rgba(255, 255, 255, 0.3);
  background: rgb(255, 255, 255);
}
.styles-module__customCheckbox___U39ax.styles-module__light___r6n4Y {
  border: 1px solid rgba(0, 0, 0, 0.15);
  background: #fff;
}
.styles-module__customCheckbox___U39ax.styles-module__light___r6n4Y.styles-module__checked___mnZLo {
  border-color: #1a1a1a;
  background: #1a1a1a;
}
.styles-module__customCheckbox___U39ax.styles-module__light___r6n4Y.styles-module__checked___mnZLo svg {
  color: #fff;
}

.styles-module__toggleLabel___Xm8Aa {
  font-size: 0.8125rem;
  font-weight: 400;
  color: rgba(255, 255, 255, 0.5);
  letter-spacing: -0.0094em;
  display: flex;
  align-items: center;
  gap: 0.25rem;
}
.styles-module__toggleLabel___Xm8Aa.styles-module__light___r6n4Y {
  color: rgba(0, 0, 0, 0.5);
}

.styles-module__helpIcon___xQg56 {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: help;
  margin-left: 0;
}
.styles-module__helpIcon___xQg56 svg {
  display: block;
  transform: translateY(1px);
  color: rgba(255, 255, 255, 0.2);
  transition: color 0.15s ease;
}
.styles-module__helpIcon___xQg56:hover svg {
  color: rgba(255, 255, 255, 0.4);
}
.styles-module__helpIcon___xQg56::after {
  content: attr(data-tooltip);
  position: absolute;
  right: calc(100% + 8px);
  top: 50%;
  transform: translateY(-50%);
  padding: 6px 10px;
  background: #383838;
  color: rgba(255, 255, 255, 0.7);
  font-size: 11px;
  font-weight: 400;
  line-height: 14px;
  border-radius: 10px;
  width: 152px;
  text-align: left;
  opacity: 0;
  visibility: hidden;
  transition: opacity 0.15s ease, visibility 0.15s ease;
  pointer-events: none;
  z-index: 100;
  box-shadow: 0px 1px 8px rgba(0, 0, 0, 0.28);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.styles-module__helpIcon___xQg56:hover::after {
  opacity: 1;
  visibility: visible;
  transition-delay: 0.5s;
}

.styles-module__dragSelection___kZLq2 {
  position: fixed;
  top: 0;
  left: 0;
  border: 2px solid rgba(52, 199, 89, 0.6);
  border-radius: 4px;
  background: rgba(52, 199, 89, 0.08);
  pointer-events: none;
  z-index: 99997;
  will-change: transform, width, height;
  contain: layout style;
}

.styles-module__dragCount___KM90j {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  background: #34c759;
  color: white;
  font-size: 0.875rem;
  font-weight: 600;
  padding: 0.25rem 0.5rem;
  border-radius: 1rem;
  min-width: 1.5rem;
  text-align: center;
}

.styles-module__highlightsContainer___-0xzG {
  position: fixed;
  top: 0;
  left: 0;
  pointer-events: none;
  z-index: 99996;
}

.styles-module__selectedElementHighlight___fyVlI {
  position: fixed;
  top: 0;
  left: 0;
  border: 2px solid rgba(52, 199, 89, 0.5);
  border-radius: 4px;
  background: rgba(52, 199, 89, 0.06);
  pointer-events: none;
  will-change: transform, width, height;
  contain: layout style;
}

.styles-module__light___r6n4Y.styles-module__toolbarContainer___dIhma {
  background: #fff;
  color: rgba(0, 0, 0, 0.85);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08), 0 4px 16px rgba(0, 0, 0, 0.06), 0 0 0 1px rgba(0, 0, 0, 0.04);
}
.styles-module__light___r6n4Y.styles-module__toolbarContainer___dIhma.styles-module__collapsed___Rydsn:hover {
  background: #f5f5f5;
}
.styles-module__light___r6n4Y.styles-module__controlButton___8Q0jc {
  color: rgba(0, 0, 0, 0.5);
}
.styles-module__light___r6n4Y.styles-module__controlButton___8Q0jc:hover:not(:disabled) {
  background: rgba(0, 0, 0, 0.06);
  color: rgba(0, 0, 0, 0.85);
}
.styles-module__light___r6n4Y.styles-module__controlButton___8Q0jc[data-active=true] {
  color: #3c82f7;
  background: rgba(60, 130, 247, 0.15);
}
.styles-module__light___r6n4Y.styles-module__controlButton___8Q0jc[data-danger]:hover:not(:disabled) {
  background: rgba(255, 59, 48, 0.15);
  color: #ff3b30;
}
.styles-module__light___r6n4Y.styles-module__buttonTooltip___Burd9 {
  background: #fff;
  color: rgba(0, 0, 0, 0.85);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08), 0 4px 16px rgba(0, 0, 0, 0.06), 0 0 0 1px rgba(0, 0, 0, 0.04);
}
.styles-module__light___r6n4Y.styles-module__buttonTooltip___Burd9::after {
  background: #fff;
}
.styles-module__light___r6n4Y.styles-module__divider___c--s1 {
  background: rgba(0, 0, 0, 0.1);
}
.styles-module__light___r6n4Y.styles-module__markerTooltip___aLJID {
  background: #fff;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.12), 0 0 0 1px rgba(0, 0, 0, 0.06);
}
.styles-module__light___r6n4Y.styles-module__markerTooltip___aLJID .styles-module__markerQuote___FHmrz {
  color: rgba(0, 0, 0, 0.5);
}
.styles-module__light___r6n4Y.styles-module__markerTooltip___aLJID .styles-module__markerNote___QkrrS {
  color: rgba(0, 0, 0, 0.85);
}
.styles-module__light___r6n4Y.styles-module__markerTooltip___aLJID .styles-module__markerHint___2iF-6 {
  color: rgba(0, 0, 0, 0.35);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y {
  background: #fff;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08), 0 4px 16px rgba(0, 0, 0, 0.06), 0 0 0 1px rgba(0, 0, 0, 0.04);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__settingsHeader___pwDY9 {
  border-bottom-color: rgba(0, 0, 0, 0.08);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__settingsBrand___0gJeM {
  color: rgba(0, 0, 0, 0.85);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__settingsBrandSlash___uTG18 {
  color: rgba(0, 0, 0, 0.4);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__settingsVersion___TUcFq {
  color: rgba(0, 0, 0, 0.4);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__settingsSection___m-YM2 {
  border-top-color: rgba(0, 0, 0, 0.08);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__settingsLabel___8UjfX {
  color: rgba(0, 0, 0, 0.5);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__cycleButton___FMKfw {
  color: rgba(0, 0, 0, 0.85);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__cycleDot___nPgLY {
  background: rgba(0, 0, 0, 0.2);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__cycleDot___nPgLY.styles-module__active___-zoN6 {
  background: rgba(0, 0, 0, 0.7);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__dropdownButton___16NPz {
  color: rgba(0, 0, 0, 0.85);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__dropdownButton___16NPz:hover {
  background: rgba(0, 0, 0, 0.05);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__toggleLabel___Xm8Aa {
  color: rgba(0, 0, 0, 0.5);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__customCheckbox___U39ax {
  border: 1px solid rgba(0, 0, 0, 0.15);
  background: #fff;
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__customCheckbox___U39ax.styles-module__checked___mnZLo {
  border-color: #1a1a1a;
  background: #1a1a1a;
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__customCheckbox___U39ax.styles-module__checked___mnZLo svg {
  color: #fff;
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__sliderLabel___U8sPr {
  color: rgba(0, 0, 0, 0.4);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__sliderLabel___U8sPr:hover {
  color: rgba(0, 0, 0, 0.7);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__sliderLabel___U8sPr.styles-module__active___-zoN6 {
  color: rgba(0, 0, 0, 0.9);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__slider___GLdxp {
  background: rgba(0, 0, 0, 0.1);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__slider___GLdxp::-webkit-slider-thumb {
  background: #1a1a1a;
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__slider___GLdxp::-moz-range-thumb {
  background: #1a1a1a;
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__helpIcon___xQg56 svg {
  color: rgba(0, 0, 0, 0.2);
}
.styles-module__light___r6n4Y.styles-module__settingsPanel___OxX3Y .styles-module__helpIcon___xQg56:hover svg {
  color: rgba(0, 0, 0, 0.4);
}

.styles-module__themeToggle___2rUjA {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  margin-left: 0.5rem;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: rgba(255, 255, 255, 0.4);
  cursor: pointer;
  transition: background-color 0.15s ease, color 0.15s ease;
}
.styles-module__themeToggle___2rUjA:hover {
  background: rgba(255, 255, 255, 0.1);
  color: rgba(255, 255, 255, 0.8);
}
.styles-module__light___r6n4Y .styles-module__themeToggle___2rUjA {
  color: rgba(0, 0, 0, 0.4);
}
.styles-module__light___r6n4Y .styles-module__themeToggle___2rUjA:hover {
  background: rgba(0, 0, 0, 0.06);
  color: rgba(0, 0, 0, 0.7);
}`,j={toolbar:`styles-module__toolbar___wNsdK`,toolbarContainer:`styles-module__toolbarContainer___dIhma`,dragging:`styles-module__dragging___xrolZ`,entrance:`styles-module__entrance___sgHd8`,toolbarEnter:`styles-module__toolbarEnter___u8RRu`,collapsed:`styles-module__collapsed___Rydsn`,expanded:`styles-module__expanded___ofKPx`,toggleContent:`styles-module__toggleContent___0yfyP`,visible:`styles-module__visible___KHwEW`,hidden:`styles-module__hidden___Ae8H4`,controlsContent:`styles-module__controlsContent___9GJWU`,badge:`styles-module__badge___2XsgF`,fadeOut:`styles-module__fadeOut___6Ut6-`,badgeEnter:`styles-module__badgeEnter___mVQLj`,controlButton:`styles-module__controlButton___8Q0jc`,buttonWrapper:`styles-module__buttonWrapper___rBcdv`,buttonTooltip:`styles-module__buttonTooltip___Burd9`,shortcut:`styles-module__shortcut___lEAQk`,tooltipBelow:`styles-module__tooltipBelow___m6ats`,tooltipsHidden:`styles-module__tooltipsHidden___VtLJG`,buttonWrapperAlignLeft:`styles-module__buttonWrapperAlignLeft___myzIp`,buttonWrapperAlignRight:`styles-module__buttonWrapperAlignRight___HCQFR`,divider:`styles-module__divider___c--s1`,overlay:`styles-module__overlay___Q1O9y`,hoverHighlight:`styles-module__hoverHighlight___ogakW`,enter:`styles-module__enter___WFIki`,hoverHighlightIn:`styles-module__hoverHighlightIn___6WYHY`,multiSelectOutline:`styles-module__multiSelectOutline___cSJ-m`,fadeIn:`styles-module__fadeIn___b9qmf`,exit:`styles-module__exit___fyOJ0`,singleSelectOutline:`styles-module__singleSelectOutline___QhX-O`,hoverTooltip:`styles-module__hoverTooltip___bvLk7`,hoverTooltipIn:`styles-module__hoverTooltipIn___FYGQx`,markersLayer:`styles-module__markersLayer___-25j1`,fixedMarkersLayer:`styles-module__fixedMarkersLayer___ffyX6`,marker:`styles-module__marker___6sQrs`,clearing:`styles-module__clearing___FQ--7`,markerIn:`styles-module__markerIn___5FaAP`,markerOut:`styles-module__markerOut___GU5jX`,pending:`styles-module__pending___2IHLC`,fixed:`styles-module__fixed___dBMHC`,multiSelect:`styles-module__multiSelect___YWiuz`,hovered:`styles-module__hovered___ZgXIy`,renumber:`styles-module__renumber___nCTxD`,renumberRoll:`styles-module__renumberRoll___Wgbq3`,markerTooltip:`styles-module__markerTooltip___aLJID`,tooltipIn:`styles-module__tooltipIn___0N31w`,markerQuote:`styles-module__markerQuote___FHmrz`,markerNote:`styles-module__markerNote___QkrrS`,markerHint:`styles-module__markerHint___2iF-6`,settingsPanel:`styles-module__settingsPanel___OxX3Y`,settingsHeader:`styles-module__settingsHeader___pwDY9`,settingsBrand:`styles-module__settingsBrand___0gJeM`,settingsBrandSlash:`styles-module__settingsBrandSlash___uTG18`,settingsVersion:`styles-module__settingsVersion___TUcFq`,settingsSection:`styles-module__settingsSection___m-YM2`,settingsLabel:`styles-module__settingsLabel___8UjfX`,cycleButton:`styles-module__cycleButton___FMKfw`,cycleDot:`styles-module__cycleDot___nPgLY`,dropdownButton:`styles-module__dropdownButton___16NPz`,toggleLabel:`styles-module__toggleLabel___Xm8Aa`,customCheckbox:`styles-module__customCheckbox___U39ax`,sliderLabel:`styles-module__sliderLabel___U8sPr`,slider:`styles-module__slider___GLdxp`,helpIcon:`styles-module__helpIcon___xQg56`,themeToggle:`styles-module__themeToggle___2rUjA`,dark:`styles-module__dark___ILIQf`,settingsOption:`styles-module__settingsOption___UNa12`,selected:`styles-module__selected___OwRqP`,settingsRow:`styles-module__settingsRow___3sdhc`,dropdownContainer:`styles-module__dropdownContainer___BVnxe`,light:`styles-module__light___r6n4Y`,cycleButtonText:`styles-module__cycleButtonText___fD1LR`,cycleTextIn:`styles-module__cycleTextIn___Q6zJf`,cycleDots:`styles-module__cycleDots___LWuoQ`,active:`styles-module__active___-zoN6`,dropdownMenu:`styles-module__dropdownMenu___k73ER`,scaleIn:`styles-module__scaleIn___c-r1K`,dropdownItem:`styles-module__dropdownItem___ylsLj`,settingsLabelMarker:`styles-module__settingsLabelMarker___ewdtV`,settingsOptions:`styles-module__settingsOptions___LyrBA`,sliderContainer:`styles-module__sliderContainer___ducXj`,sliderLabels:`styles-module__sliderLabels___FhLDB`,colorOptions:`styles-module__colorOptions___iHCNX`,colorOption:`styles-module__colorOption___IodiY`,colorOptionRing:`styles-module__colorOptionRing___U2xpo`,settingsToggle:`styles-module__settingsToggle___fBrFn`,checked:`styles-module__checked___mnZLo`,dragSelection:`styles-module__dragSelection___kZLq2`,dragCount:`styles-module__dragCount___KM90j`,highlightsContainer:`styles-module__highlightsContainer___-0xzG`,selectedElementHighlight:`styles-module__selectedElementHighlight___fyVlI`,scaleOut:`styles-module__scaleOut___Wctwz`,slideUp:`styles-module__slideUp___kgD36`,slideDown:`styles-module__slideDown___zcdje`,settingsPanelIn:`styles-module__settingsPanelIn___MGfO8`,settingsPanelOut:`styles-module__settingsPanelOut___Zfymi`};if(typeof document<`u`){let e=document.getElementById(`feedback-tool-styles-page-toolbar-css-styles`);e||(e=document.createElement(`style`),e.id=`feedback-tool-styles-page-toolbar-css-styles`,e.textContent=A,document.head.appendChild(e))}var M=j,ye=!1,be={outputDetail:`standard`,autoClearAfterCopy:!1,annotationColor:`#3c82f7`,blockInteractions:!1},N=[{value:`compact`,label:`Compact`},{value:`standard`,label:`Standard`},{value:`detailed`,label:`Detailed`},{value:`forensic`,label:`Forensic`}],xe=[{value:`#AF52DE`,label:`Purple`},{value:`#3c82f7`,label:`Blue`},{value:`#5AC8FA`,label:`Cyan`},{value:`#34C759`,label:`Green`},{value:`#FFD60A`,label:`Yellow`},{value:`#FF9500`,label:`Orange`},{value:`#FF3B30`,label:`Red`}];function Se(e){let t=e;for(;t&&t!==document.body;){let e=window.getComputedStyle(t).position;if(e===`fixed`||e===`sticky`)return!0;t=t.parentElement}return!1}function Ce(e,t,n=`standard`){if(e.length===0)return``;let r=typeof window<`u`?`${window.innerWidth}\xD7${window.innerHeight}`:`unknown`,i=`## Page Feedback: ${t}
`;return n===`forensic`?(i+=`
**Environment:**
`,i+=`- Viewport: ${r}
`,typeof window<`u`&&(i+=`- URL: ${window.location.href}
`,i+=`- User Agent: ${navigator.userAgent}
`,i+=`- Timestamp: ${new Date().toISOString()}
`,i+=`- Device Pixel Ratio: ${window.devicePixelRatio}
`),i+=`
---
`):n!==`compact`&&(i+=`**Viewport:** ${r}
`),i+=`
`,e.forEach((e,t)=>{n===`compact`?(i+=`${t+1}. **${e.element}**: ${e.comment}`,e.selectedText&&(i+=` (re: "${e.selectedText.slice(0,30)}${e.selectedText.length>30?`...`:``}")`),i+=`
`):n===`forensic`?(i+=`### ${t+1}. ${e.element}
`,e.isMultiSelect&&e.fullPath&&(i+=`*Forensic data shown for first element of selection*
`),e.fullPath&&(i+=`**Full DOM Path:** ${e.fullPath}
`),e.cssClasses&&(i+=`**CSS Classes:** ${e.cssClasses}
`),e.boundingBox&&(i+=`**Position:** x:${Math.round(e.boundingBox.x)}, y:${Math.round(e.boundingBox.y)} (${Math.round(e.boundingBox.width)}\xD7${Math.round(e.boundingBox.height)}px)
`),i+=`**Annotation at:** ${e.x.toFixed(1)}% from left, ${Math.round(e.y)}px from top
`,e.selectedText&&(i+=`**Selected text:** "${e.selectedText}"
`),e.nearbyText&&!e.selectedText&&(i+=`**Context:** ${e.nearbyText.slice(0,100)}
`),e.computedStyles&&(i+=`**Computed Styles:** ${e.computedStyles}
`),e.accessibility&&(i+=`**Accessibility:** ${e.accessibility}
`),e.nearbyElements&&(i+=`**Nearby Elements:** ${e.nearbyElements}
`),i+=`**Feedback:** ${e.comment}

`):(i+=`### ${t+1}. ${e.element}
`,i+=`**Location:** ${e.elementPath}
`,n===`detailed`&&(e.cssClasses&&(i+=`**Classes:** ${e.cssClasses}
`),e.boundingBox&&(i+=`**Position:** ${Math.round(e.boundingBox.x)}px, ${Math.round(e.boundingBox.y)}px (${Math.round(e.boundingBox.width)}\xD7${Math.round(e.boundingBox.height)}px)
`)),e.selectedText&&(i+=`**Selected text:** "${e.selectedText}"
`),n===`detailed`&&e.nearbyText&&!e.selectedText&&(i+=`**Context:** ${e.nearbyText.slice(0,100)}
`),i+=`**Feedback:** ${e.comment}

`)}),i.trim()}function we({demoAnnotations:e,demoDelay:t=1e3,enableDemoMode:n=!1,onAnnotationAdd:r,onAnnotationDelete:i,onAnnotationUpdate:a,onAnnotationsClear:o,onCopy:u,copyToClipboard:d=!0}={}){let[f,x]=(0,s.useState)(!1),[T,E]=(0,s.useState)([]),[D,ce]=(0,s.useState)(!0),[O,ue]=(0,s.useState)(!1),[k,he]=(0,s.useState)(!1),[A,j]=(0,s.useState)(null),[we,Te]=(0,s.useState)({x:0,y:0}),[P,F]=(0,s.useState)(null),[Ee,De]=(0,s.useState)(!1),[Oe,ke]=(0,s.useState)(!1),[Ae,je]=(0,s.useState)(!1),[I,L]=(0,s.useState)(null),[Me,Ne]=(0,s.useState)(null),[R,Pe]=(0,s.useState)(null),[z,B]=(0,s.useState)(null),[Fe,Ie]=(0,s.useState)(0),[Le,Re]=(0,s.useState)(!1),[V,ze]=(0,s.useState)(!1),[H,Be]=(0,s.useState)(!1),[U,Ve]=(0,s.useState)(!1),[He,Ue]=(0,s.useState)(!1),[We,Ge]=(0,s.useState)(!1),W=()=>{Ge(!0)},Ke=()=>{Ge(!1)},[G,qe]=(0,s.useState)(be),[K,Je]=(0,s.useState)(!0),[Ye,Xe]=(0,s.useState)(!1),[q,Ze]=(0,s.useState)(null),[J,Qe]=(0,s.useState)(!1),[Y,$e]=(0,s.useState)(null),[et,tt]=(0,s.useState)(0),nt=(0,s.useRef)(!1),[rt,it]=(0,s.useState)(new Set),[at,ot]=(0,s.useState)(new Set),[st,ct]=(0,s.useState)(!1),[lt,ut]=(0,s.useState)(!1),[X,dt]=(0,s.useState)(!1),Z=(0,s.useRef)(null),Q=(0,s.useRef)(null),ft=(0,s.useRef)(null),pt=(0,s.useRef)(null),mt=(0,s.useRef)(!1),ht=(0,s.useRef)(0),gt=(0,s.useRef)(null),_t=(0,s.useRef)(null),vt=(0,s.useRef)(null),yt=(0,s.useRef)(null),$=typeof window<`u`?window.location.pathname:`/`;(0,s.useEffect)(()=>{if(U)Ue(!0);else{let e=setTimeout(()=>Ue(!1),0);return()=>clearTimeout(e)}},[U]);let bt=f&&D;(0,s.useEffect)(()=>{if(bt){he(!1),ue(!0),it(new Set);let e=setTimeout(()=>{it(e=>{let t=new Set(e);return T.forEach(e=>t.add(e.id)),t})},350);return()=>clearTimeout(e)}if(O){he(!0);let e=setTimeout(()=>{ue(!1),he(!1)},250);return()=>clearTimeout(e)}},[bt]),(0,s.useEffect)(()=>{ze(!0),Ie(window.scrollY);let e=_e($);E(e),ye||(Xe(!0),ye=!0,setTimeout(()=>Xe(!1),750));try{let e=localStorage.getItem(`feedback-toolbar-settings`);e&&qe({...be,...JSON.parse(e)})}catch{}try{let e=localStorage.getItem(`feedback-toolbar-theme`);e!==null&&Je(e===`dark`)}catch{}},[$]),(0,s.useEffect)(()=>{V&&localStorage.setItem(`feedback-toolbar-settings`,JSON.stringify(G))},[G,V]),(0,s.useEffect)(()=>{V&&localStorage.setItem(`feedback-toolbar-theme`,K?`dark`:`light`)},[K,V]),(0,s.useEffect)(()=>{if(!n||!V||!e||e.length===0||T.length>0)return;let r=[];return r.push(setTimeout(()=>{x(!0)},t-200)),e.forEach((e,n)=>{let i=t+n*300;r.push(setTimeout(()=>{let t=document.querySelector(e.selector);if(!t)return;let r=t.getBoundingClientRect(),{name:i,path:a}=S(t),o={id:`demo-${Date.now()}-${n}`,x:(r.left+r.width/2)/window.innerWidth*100,y:r.top+r.height/2+window.scrollY,comment:e.comment,element:i,elementPath:a,timestamp:Date.now(),selectedText:e.selectedText,boundingBox:{x:r.left,y:r.top+window.scrollY,width:r.width,height:r.height},nearbyText:C(t),cssClasses:se(t)};E(e=>[...e,o])},i))}),()=>{r.forEach(clearTimeout)}},[n,V,e,t]),(0,s.useEffect)(()=>{let e=()=>{Ie(window.scrollY),Re(!0),yt.current&&clearTimeout(yt.current),yt.current=setTimeout(()=>{Re(!1)},150)};return window.addEventListener(`scroll`,e,{passive:!0}),()=>{window.removeEventListener(`scroll`,e),yt.current&&clearTimeout(yt.current)}},[]),(0,s.useEffect)(()=>{V&&T.length>0?ve($,T):V&&T.length===0&&localStorage.removeItem(ge($))},[T,$,V]);let xt=(0,s.useCallback)(()=>{if(H)return;let e=document.createElement(`style`);e.id=`feedback-freeze-styles`,e.textContent=`
      *:not([data-feedback-toolbar]):not([data-feedback-toolbar] *):not([data-annotation-popup]):not([data-annotation-popup] *):not([data-annotation-marker]):not([data-annotation-marker] *),
      *:not([data-feedback-toolbar]):not([data-feedback-toolbar] *):not([data-annotation-popup]):not([data-annotation-popup] *):not([data-annotation-marker]):not([data-annotation-marker] *)::before,
      *:not([data-feedback-toolbar]):not([data-feedback-toolbar] *):not([data-annotation-popup]):not([data-annotation-popup] *):not([data-annotation-marker]):not([data-annotation-marker] *)::after {
        animation-play-state: paused !important;
        transition: none !important;
      }
    `,document.head.appendChild(e),document.querySelectorAll(`video`).forEach(e=>{e.paused||(e.dataset.wasPaused=`false`,e.pause())}),Be(!0)},[H]),St=(0,s.useCallback)(()=>{if(!H)return;let e=document.getElementById(`feedback-freeze-styles`);e&&e.remove(),document.querySelectorAll(`video`).forEach(e=>{e.dataset.wasPaused===`false`&&(e.play(),delete e.dataset.wasPaused)}),Be(!1)},[H]),Ct=(0,s.useCallback)(()=>{H?St():xt()},[H,xt,St]);(0,s.useEffect)(()=>{f||(F(null),B(null),j(null),Ve(!1),H&&St())},[f,H,St]),(0,s.useEffect)(()=>{if(!f)return;let e=document.createElement(`style`);return e.id=`feedback-cursor-styles`,e.textContent=`
      body * {
        cursor: crosshair !important;
      }
      body p, body span, body h1, body h2, body h3, body h4, body h5, body h6,
      body li, body td, body th, body label, body blockquote, body figcaption,
      body caption, body legend, body dt, body dd, body pre, body code,
      body em, body strong, body b, body i, body u, body s, body a,
      body time, body address, body cite, body q, body abbr, body dfn,
      body mark, body small, body sub, body sup, body [contenteditable],
      body p *, body span *, body h1 *, body h2 *, body h3 *, body h4 *,
      body h5 *, body h6 *, body li *, body a *, body label *, body pre *,
      body code *, body blockquote *, body [contenteditable] * {
        cursor: text !important;
      }
      [data-feedback-toolbar], [data-feedback-toolbar] * {
        cursor: default !important;
      }
      [data-annotation-marker], [data-annotation-marker] * {
        cursor: pointer !important;
      }
    `,document.head.appendChild(e),()=>{let e=document.getElementById(`feedback-cursor-styles`);e&&e.remove()}},[f]),(0,s.useEffect)(()=>{if(!f||P)return;let e=e=>{if(e.target.closest(`[data-feedback-toolbar]`)){j(null);return}let t=document.elementFromPoint(e.clientX,e.clientY);if(!t||t.closest(`[data-feedback-toolbar]`)){j(null);return}let{name:n,path:r}=S(t),i=t.getBoundingClientRect();j({element:n,elementPath:r,rect:i}),Te({x:e.clientX,y:e.clientY})};return document.addEventListener(`mousemove`,e),()=>document.removeEventListener(`mousemove`,e)},[f,P]),(0,s.useEffect)(()=>{if(!f)return;let e=e=>{if(mt.current){mt.current=!1;return}let t=e.target;if(t.closest(`[data-feedback-toolbar]`)||t.closest(`[data-annotation-popup]`)||t.closest(`[data-annotation-marker]`))return;let n=t.closest(`button, a, input, select, textarea, [role='button'], [onclick]`);if(G.blockInteractions&&n&&(e.preventDefault(),e.stopPropagation()),P){if(n&&!G.blockInteractions)return;e.preventDefault(),_t.current?.shake();return}if(z){if(n&&!G.blockInteractions)return;e.preventDefault(),vt.current?.shake();return}e.preventDefault();let r=document.elementFromPoint(e.clientX,e.clientY);if(!r)return;let{name:i,path:a}=S(r),o=r.getBoundingClientRect(),s=e.clientX/window.innerWidth*100,c=Se(r),l=c?e.clientY:e.clientY+window.scrollY,u=window.getSelection(),d;u&&u.toString().trim().length>0&&(d=u.toString().trim().slice(0,500));let f=le(r),p=de(r);F({x:s,y:l,clientY:e.clientY,element:i,elementPath:a,selectedText:d,boundingBox:{x:o.left,y:c?o.top:o.top+window.scrollY,width:o.width,height:o.height},nearbyText:C(r),cssClasses:se(r),isFixed:c,fullPath:me(r),accessibility:pe(r),computedStyles:p,computedStylesObj:f,nearbyElements:w(r)}),j(null)};return document.addEventListener(`click`,e,!0),()=>document.removeEventListener(`click`,e,!0)},[f,P,z,G.blockInteractions]),(0,s.useEffect)(()=>{if(!f||P)return;let e=e=>{let t=e.target;t.closest(`[data-feedback-toolbar]`)||t.closest(`[data-annotation-marker]`)||t.closest(`[data-annotation-popup]`)||new Set(`P.SPAN.H1.H2.H3.H4.H5.H6.LI.TD.TH.LABEL.BLOCKQUOTE.FIGCAPTION.CAPTION.LEGEND.DT.DD.PRE.CODE.EM.STRONG.B.I.U.S.A.TIME.ADDRESS.CITE.Q.ABBR.DFN.MARK.SMALL.SUB.SUP`.split(`.`)).has(t.tagName)||t.isContentEditable||(Z.current={x:e.clientX,y:e.clientY})};return document.addEventListener(`mousedown`,e),()=>document.removeEventListener(`mousedown`,e)},[f,P]),(0,s.useEffect)(()=>{if(!f||P)return;let e=e=>{if(!Z.current)return;let t=e.clientX-Z.current.x,n=e.clientY-Z.current.y,r=t*t+n*n;if(!X&&r>=64&&(Q.current=Z.current,dt(!0)),(X||r>=64)&&Q.current){if(ft.current){let t=Math.min(Q.current.x,e.clientX),n=Math.min(Q.current.y,e.clientY),r=Math.abs(e.clientX-Q.current.x),i=Math.abs(e.clientY-Q.current.y);ft.current.style.transform=`translate(${t}px, ${n}px)`,ft.current.style.width=`${r}px`,ft.current.style.height=`${i}px`}let t=Date.now();if(t-ht.current<50)return;ht.current=t;let n=Q.current.x,r=Q.current.y,i=Math.min(n,e.clientX),a=Math.min(r,e.clientY),o=Math.max(n,e.clientX),s=Math.max(r,e.clientY),c=(i+o)/2,l=(a+s)/2,u=new Set,d=[[i,a],[o,a],[i,s],[o,s],[c,l],[c,a],[c,s],[i,l],[o,l]];for(let[e,t]of d){let n=document.elementsFromPoint(e,t);for(let e of n)e instanceof HTMLElement&&u.add(e)}let f=document.querySelectorAll(`button, a, input, img, p, h1, h2, h3, h4, h5, h6, li, label, td, th, div, span, section, article, aside, nav`);for(let e of f)if(e instanceof HTMLElement){let t=e.getBoundingClientRect(),n=t.left+t.width/2,r=t.top+t.height/2,c=n>=i&&n<=o&&r>=a&&r<=s,l=Math.min(t.right,o)-Math.max(t.left,i),d=Math.min(t.bottom,s)-Math.max(t.top,a),f=l>0&&d>0?l*d:0,p=t.width*t.height,m=p>0?f/p:0;(c||m>.5)&&u.add(e)}let p=[],m=new Set([`BUTTON`,`A`,`INPUT`,`IMG`,`P`,`H1`,`H2`,`H3`,`H4`,`H5`,`H6`,`LI`,`LABEL`,`TD`,`TH`,`SECTION`,`ARTICLE`,`ASIDE`,`NAV`]);for(let e of u){if(e.closest(`[data-feedback-toolbar]`)||e.closest(`[data-annotation-marker]`))continue;let t=e.getBoundingClientRect();if(!(t.width>window.innerWidth*.8&&t.height>window.innerHeight*.5)&&!(t.width<10||t.height<10)&&t.left<o&&t.right>i&&t.top<s&&t.bottom>a){let n=e.tagName,r=m.has(n);if(!r&&(n===`DIV`||n===`SPAN`)){let t=e.textContent&&e.textContent.trim().length>0,n=e.onclick!==null||e.getAttribute(`role`)===`button`||e.getAttribute(`role`)===`link`||e.classList.contains(`clickable`)||e.hasAttribute(`data-clickable`);(t||n)&&!e.querySelector(`p, h1, h2, h3, h4, h5, h6, button, a`)&&(r=!0)}if(r){let e=!1;for(let n of p)if(n.left<=t.left&&n.right>=t.right&&n.top<=t.top&&n.bottom>=t.bottom){e=!0;break}e||p.push(t)}}}if(pt.current){let e=pt.current;for(;e.children.length>p.length;)e.removeChild(e.lastChild);p.forEach((t,n)=>{let r=e.children[n];r||(r=document.createElement(`div`),r.className=M.selectedElementHighlight,e.appendChild(r)),r.style.transform=`translate(${t.left}px, ${t.top}px)`,r.style.width=`${t.width}px`,r.style.height=`${t.height}px`})}}};return document.addEventListener(`mousemove`,e,{passive:!0}),()=>document.removeEventListener(`mousemove`,e)},[f,P,X,8]),(0,s.useEffect)(()=>{if(!f)return;let e=e=>{let t=X,n=Q.current;if(X&&n){mt.current=!0;let t=Math.min(n.x,e.clientX),r=Math.min(n.y,e.clientY),i=Math.max(n.x,e.clientX),a=Math.max(n.y,e.clientY),o=[];document.querySelectorAll(`button, a, input, img, p, h1, h2, h3, h4, h5, h6, li, label, td, th`).forEach(e=>{if(!(e instanceof HTMLElement)||e.closest(`[data-feedback-toolbar]`)||e.closest(`[data-annotation-marker]`))return;let n=e.getBoundingClientRect();n.width>window.innerWidth*.8&&n.height>window.innerHeight*.5||n.width<10||n.height<10||n.left<i&&n.right>t&&n.top<a&&n.bottom>r&&o.push({element:e,rect:n})});let s=o.filter(({element:e})=>!o.some(({element:t})=>t!==e&&e.contains(t))),c=e.clientX/window.innerWidth*100,l=e.clientY+window.scrollY;if(s.length>0){let t=s.reduce((e,{rect:t})=>({left:Math.min(e.left,t.left),top:Math.min(e.top,t.top),right:Math.max(e.right,t.right),bottom:Math.max(e.bottom,t.bottom)}),{left:1/0,top:1/0,right:-1/0,bottom:-1/0}),n=s.slice(0,5).map(({element:e})=>S(e).name).join(`, `),r=s.length>5?` +${s.length-5} more`:``,i=s[0].element,a=le(i),o=de(i);F({x:c,y:l,clientY:e.clientY,element:`${s.length} elements: ${n}${r}`,elementPath:`multi-select`,boundingBox:{x:t.left,y:t.top+window.scrollY,width:t.right-t.left,height:t.bottom-t.top},isMultiSelect:!0,fullPath:me(i),accessibility:pe(i),computedStyles:o,computedStylesObj:a,nearbyElements:w(i),cssClasses:se(i),nearbyText:C(i)})}else{let n=Math.abs(i-t),o=Math.abs(a-r);n>20&&o>20&&F({x:c,y:l,clientY:e.clientY,element:`Area selection`,elementPath:`region at (${Math.round(t)}, ${Math.round(r)})`,boundingBox:{x:t,y:r+window.scrollY,width:n,height:o},isMultiSelect:!0})}j(null)}else t&&(mt.current=!0);Z.current=null,Q.current=null,dt(!1),pt.current&&(pt.current.innerHTML=``)};return document.addEventListener(`mouseup`,e),()=>document.removeEventListener(`mouseup`,e)},[f,X]);let wt=(0,s.useCallback)(e=>{if(!P)return;let t={id:Date.now().toString(),x:P.x,y:P.y,comment:e,element:P.element,elementPath:P.elementPath,timestamp:Date.now(),selectedText:P.selectedText,boundingBox:P.boundingBox,nearbyText:P.nearbyText,cssClasses:P.cssClasses,isMultiSelect:P.isMultiSelect,isFixed:P.isFixed,fullPath:P.fullPath,accessibility:P.accessibility,computedStyles:P.computedStyles,nearbyElements:P.nearbyElements};E(e=>[...e,t]),gt.current=t.id,setTimeout(()=>{gt.current=null},300),setTimeout(()=>{it(e=>new Set(e).add(t.id))},250),r?.(t),ct(!0),setTimeout(()=>{F(null),ct(!1)},150),window.getSelection()?.removeAllRanges()},[P,r]),Tt=(0,s.useCallback)(()=>{ct(!0),setTimeout(()=>{F(null),ct(!1)},150)},[]),Et=(0,s.useCallback)(e=>{let t=T.findIndex(t=>t.id===e),n=T[t];Ne(e),ot(t=>new Set(t).add(e)),n&&i?.(n),setTimeout(()=>{E(t=>t.filter(t=>t.id!==e)),ot(t=>{let n=new Set(t);return n.delete(e),n}),Ne(null),t<T.length-1&&(Pe(t),setTimeout(()=>Pe(null),200))},150)},[T,i]),Dt=(0,s.useCallback)(e=>{B(e),L(null)},[]),Ot=(0,s.useCallback)(e=>{if(!z)return;let t={...z,comment:e};E(e=>e.map(e=>e.id===z.id?t:e)),a?.(t),ut(!0),setTimeout(()=>{B(null),ut(!1)},150)},[z,a]),kt=(0,s.useCallback)(()=>{ut(!0),setTimeout(()=>{B(null),ut(!1)},150)},[]),At=(0,s.useCallback)(()=>{let e=T.length;if(e===0)return;o?.(T),je(!0),ke(!0);let t=e*30+200;setTimeout(()=>{E([]),it(new Set),localStorage.removeItem(ge($)),je(!1)},t),setTimeout(()=>ke(!1),1500)},[$,T,o]),jt=(0,s.useCallback)(async()=>{let e=Ce(T,$,G.outputDetail);if(e){if(d)try{await navigator.clipboard.writeText(e)}catch{}u?.(e),De(!0),setTimeout(()=>De(!1),2e3),G.autoClearAfterCopy&&setTimeout(()=>At(),500)}},[T,$,G.outputDetail,G.autoClearAfterCopy,At,d,u]);(0,s.useEffect)(()=>{if(!Y)return;let e=e=>{let t=e.clientX-Y.x,n=e.clientY-Y.y,r=Math.sqrt(t*t+n*n);if(!J&&r>5&&Qe(!0),J||r>5){let e=Y.toolbarX+t,r=Y.toolbarY+n;if(f)e=Math.max(20,Math.min(window.innerWidth-257-20,e));else{let t=window.innerWidth-20-213-44;e=Math.max(-193,Math.min(t,e))}r=Math.max(20,Math.min(window.innerHeight-44-20,r)),Ze({x:e,y:r})}},t=()=>{J&&(nt.current=!0,setTimeout(()=>{nt.current=!1},50)),Qe(!1),$e(null)};return document.addEventListener(`mousemove`,e),document.addEventListener(`mouseup`,t),()=>{document.removeEventListener(`mousemove`,e),document.removeEventListener(`mouseup`,t)}},[Y,J,f]);let Mt=(0,s.useCallback)(e=>{if(e.target.closest(`button`)||e.target.closest(`.${M.settingsPanel}`))return;let t=e.currentTarget.parentElement;if(!t)return;let n=t.getBoundingClientRect(),r=q?.x??n.left,i=q?.y??n.top,a=(Math.random()-.5)*10;tt(a),$e({x:e.clientX,y:e.clientY,toolbarX:r,toolbarY:i})},[q]);if((0,s.useEffect)(()=>{if(!q)return;let e=()=>{let e=q.x,t=q.y;if(f)e=Math.max(20,Math.min(window.innerWidth-257-20,e));else{let t=window.innerWidth-20-213-44;e=Math.max(-193,Math.min(t,e))}t=Math.max(20,Math.min(window.innerHeight-44-20,t)),(e!==q.x||t!==q.y)&&Ze({x:e,y:t})};return e(),window.addEventListener(`resize`,e),()=>window.removeEventListener(`resize`,e)},[q,f]),(0,s.useEffect)(()=>{let e=e=>{e.key===`Escape`&&(P||f&&x(!1))};return document.addEventListener(`keydown`,e),()=>document.removeEventListener(`keydown`,e)},[f,P]),!V)return null;let Nt=T.length>0,Pt=T.filter(e=>!at.has(e.id)),Ft=T.filter(e=>at.has(e.id)),It=e=>{let t=e.x/100*window.innerWidth,n=typeof e.y==`string`?parseFloat(e.y):e.y,r={};window.innerHeight-n-22-10<80&&(r.top=`auto`,r.bottom=`calc(100% + 10px)`);let i=t-100;return i<10?r.left=`calc(50% + ${10-i}px)`:i+200>window.innerWidth-10&&(r.left=`calc(50% - ${i+200-(window.innerWidth-10)}px)`),r};return(0,c.createPortal)((0,l.jsxs)(l.Fragment,{children:[(0,l.jsx)(`div`,{className:M.toolbar,"data-feedback-toolbar":!0,style:q?{left:q.x,top:q.y,right:`auto`,bottom:`auto`}:void 0,children:(0,l.jsxs)(`div`,{className:`${M.toolbarContainer} ${K?``:M.light} ${f?M.expanded:M.collapsed} ${Ye?M.entrance:``} ${J?M.dragging:``}`,onClick:f?void 0:e=>{if(nt.current){e.preventDefault();return}x(!0)},onMouseDown:Mt,role:f?void 0:`button`,tabIndex:f?-1:0,title:f?void 0:`Start feedback mode`,style:J?{transform:`scale(1.05) rotate(${et}deg)`,cursor:`grabbing`}:void 0,children:[(0,l.jsxs)(`div`,{className:`${M.toggleContent} ${f?M.hidden:M.visible}`,children:[(0,l.jsx)(ee,{size:24}),Nt&&(0,l.jsx)(`span`,{className:`${M.badge} ${f?M.fadeOut:``} ${Ye?M.entrance:``}`,style:{backgroundColor:G.annotationColor},children:T.length})]}),(0,l.jsxs)(`div`,{className:`${M.controlsContent} ${f?M.visible:M.hidden} ${q&&q.y<100?M.tooltipBelow:``} ${We||U?M.tooltipsHidden:``}`,onMouseLeave:Ke,children:[(0,l.jsxs)(`div`,{className:`${M.buttonWrapper} ${q&&q.x<120?M.buttonWrapperAlignLeft:``}`,children:[(0,l.jsx)(`button`,{className:`${M.controlButton} ${K?``:M.light}`,onClick:e=>{e.stopPropagation(),W(),Ct()},"data-active":H,children:(0,l.jsx)(v,{size:24,isPaused:H})}),(0,l.jsxs)(`span`,{className:M.buttonTooltip,children:[H?`Resume animations`:`Pause animations`,(0,l.jsx)(`span`,{className:M.shortcut,children:`P`})]})]}),(0,l.jsxs)(`div`,{className:M.buttonWrapper,children:[(0,l.jsx)(`button`,{className:`${M.controlButton} ${K?``:M.light}`,onClick:e=>{e.stopPropagation(),W(),ce(!D)},disabled:!Nt,children:(0,l.jsx)(ne,{size:24,isOpen:D})}),(0,l.jsxs)(`span`,{className:M.buttonTooltip,children:[D?`Hide markers`:`Show markers`,(0,l.jsx)(`span`,{className:M.shortcut,children:`H`})]})]}),(0,l.jsxs)(`div`,{className:M.buttonWrapper,children:[(0,l.jsx)(`button`,{className:`${M.controlButton} ${K?``:M.light}`,onClick:e=>{e.stopPropagation(),W(),jt()},disabled:!Nt,"data-active":Ee,children:(0,l.jsx)(_,{size:24,copied:Ee})}),(0,l.jsxs)(`span`,{className:M.buttonTooltip,children:[`Copy feedback`,(0,l.jsx)(`span`,{className:M.shortcut,children:`C`})]})]}),(0,l.jsxs)(`div`,{className:M.buttonWrapper,children:[(0,l.jsx)(`button`,{className:`${M.controlButton} ${K?``:M.light}`,onClick:e=>{e.stopPropagation(),W(),At()},disabled:!Nt,"data-danger":!0,children:(0,l.jsx)(re,{size:24})}),(0,l.jsxs)(`span`,{className:M.buttonTooltip,children:[`Clear all`,(0,l.jsx)(`span`,{className:M.shortcut,children:`X`})]})]}),(0,l.jsxs)(`div`,{className:M.buttonWrapper,children:[(0,l.jsx)(`button`,{className:`${M.controlButton} ${K?``:M.light}`,onClick:e=>{e.stopPropagation(),W(),Ve(!U)},children:(0,l.jsx)(y,{size:24})}),(0,l.jsx)(`span`,{className:M.buttonTooltip,children:`Settings`})]}),(0,l.jsx)(`div`,{className:`${M.divider} ${K?``:M.light}`}),(0,l.jsxs)(`div`,{className:`${M.buttonWrapper} ${q&&typeof window<`u`&&q.x>window.innerWidth-120?M.buttonWrapperAlignRight:``}`,children:[(0,l.jsx)(`button`,{className:`${M.controlButton} ${K?``:M.light}`,onClick:e=>{e.stopPropagation(),W(),x(!1)},children:(0,l.jsx)(ae,{size:24})}),(0,l.jsxs)(`span`,{className:M.buttonTooltip,children:[`Exit`,(0,l.jsx)(`span`,{className:M.shortcut,children:`Esc`})]})]})]}),(0,l.jsxs)(`div`,{className:`${M.settingsPanel} ${K?M.dark:M.light} ${He?M.enter:M.exit}`,onClick:e=>e.stopPropagation(),style:q&&q.y<230?{bottom:`auto`,top:`calc(100% + 0.5rem)`}:void 0,children:[(0,l.jsxs)(`div`,{className:M.settingsHeader,children:[(0,l.jsxs)(`span`,{className:M.settingsBrand,children:[(0,l.jsx)(`span`,{className:M.settingsBrandSlash,style:{color:G.annotationColor,transition:`color 0.2s ease`},children:`/`}),`agentation`]}),(0,l.jsxs)(`span`,{className:M.settingsVersion,children:[`v`,`1.3.1`]}),(0,l.jsx)(`button`,{className:M.themeToggle,onClick:()=>Je(!K),title:K?`Switch to light mode`:`Switch to dark mode`,children:K?(0,l.jsx)(oe,{size:14}):(0,l.jsx)(b,{size:14})})]}),(0,l.jsx)(`div`,{className:M.settingsSection,children:(0,l.jsxs)(`div`,{className:M.settingsRow,children:[(0,l.jsxs)(`div`,{className:`${M.settingsLabel} ${K?``:M.light}`,children:[`Output Detail`,(0,l.jsx)(`span`,{className:M.helpIcon,"data-tooltip":`Controls how much detail is included in the copied output`,children:(0,l.jsx)(g,{size:20})})]}),(0,l.jsxs)(`button`,{className:`${M.cycleButton} ${K?``:M.light}`,onClick:()=>{let e=(N.findIndex(e=>e.value===G.outputDetail)+1)%N.length;qe(t=>({...t,outputDetail:N[e].value}))},children:[(0,l.jsx)(`span`,{className:M.cycleButtonText,children:N.find(e=>e.value===G.outputDetail)?.label},G.outputDetail),(0,l.jsx)(`span`,{className:M.cycleDots,children:N.map((e,t)=>(0,l.jsx)(`span`,{className:`${M.cycleDot} ${K?``:M.light} ${G.outputDetail===e.value?M.active:``}`},e.value))})]})]})}),(0,l.jsxs)(`div`,{className:M.settingsSection,children:[(0,l.jsx)(`div`,{className:`${M.settingsLabel} ${M.settingsLabelMarker} ${K?``:M.light}`,children:`Marker Colour`}),(0,l.jsx)(`div`,{className:M.colorOptions,children:xe.map(e=>(0,l.jsx)(`div`,{onClick:()=>qe(t=>({...t,annotationColor:e.value})),style:{borderColor:G.annotationColor===e.value?e.value:`transparent`},className:`${M.colorOptionRing} ${G.annotationColor===e.value?M.selected:``}`,children:(0,l.jsx)(`div`,{className:`${M.colorOption} ${G.annotationColor===e.value?M.selected:``}`,style:{backgroundColor:e.value},title:e.label})},e.value))})]}),(0,l.jsxs)(`div`,{className:M.settingsSection,children:[(0,l.jsxs)(`label`,{className:M.settingsToggle,children:[(0,l.jsx)(`input`,{type:`checkbox`,id:`autoClearAfterCopy`,checked:G.autoClearAfterCopy,onChange:e=>qe(t=>({...t,autoClearAfterCopy:e.target.checked}))}),(0,l.jsx)(`label`,{className:`${M.customCheckbox} ${G.autoClearAfterCopy?M.checked:``}`,htmlFor:`autoClearAfterCopy`,children:G.autoClearAfterCopy&&(0,l.jsx)(te,{size:14})}),(0,l.jsxs)(`span`,{className:`${M.toggleLabel} ${K?``:M.light}`,children:[`Clear after output`,(0,l.jsx)(`span`,{className:M.helpIcon,"data-tooltip":`Automatically clear annotations after copying`,children:(0,l.jsx)(g,{size:20})})]})]}),(0,l.jsxs)(`label`,{className:M.settingsToggle,children:[(0,l.jsx)(`input`,{type:`checkbox`,id:`blockInteractions`,checked:G.blockInteractions,onChange:e=>qe(t=>({...t,blockInteractions:e.target.checked}))}),(0,l.jsx)(`label`,{className:`${M.customCheckbox} ${G.blockInteractions?M.checked:``}`,htmlFor:`blockInteractions`,children:G.blockInteractions&&(0,l.jsx)(te,{size:14})}),(0,l.jsx)(`span`,{className:`${M.toggleLabel} ${K?``:M.light}`,children:`Block page interactions`})]})]})]})]})}),(0,l.jsxs)(`div`,{className:M.markersLayer,"data-feedback-toolbar":!0,children:[O&&Pt.filter(e=>!e.isFixed).map((e,t)=>{let n=!k&&I===e.id,r=Me===e.id,i=n||r,a=e.isMultiSelect,o=a?`#34C759`:G.annotationColor,s=T.findIndex(t=>t.id===e.id),c=!rt.has(e.id),u=k?M.exit:Ae?M.clearing:c?M.enter:``;return(0,l.jsxs)(`div`,{className:`${M.marker} ${i?M.hovered:``} ${a?M.multiSelect:``} ${u}`,"data-annotation-marker":!0,style:{left:`${e.x}%`,top:e.y,backgroundColor:i?void 0:o,animationDelay:k?`${(Pt.length-1-t)*20}ms`:`${t*20}ms`},onMouseEnter:()=>!k&&e.id!==gt.current&&L(e.id),onMouseLeave:()=>L(null),onClick:t=>{t.stopPropagation(),k||Et(e.id)},onContextMenu:t=>{t.preventDefault(),t.stopPropagation(),k||Dt(e)},children:[i?(0,l.jsx)(ie,{size:a?18:16}):(0,l.jsx)(`span`,{className:R!==null&&s>=R?M.renumber:void 0,children:s+1}),n&&!z&&(0,l.jsxs)(`div`,{className:`${M.markerTooltip} ${K?``:M.light} ${M.enter}`,style:It(e),children:[(0,l.jsxs)(`span`,{className:M.markerQuote,children:[e.element,e.selectedText&&` "${e.selectedText.slice(0,30)}${e.selectedText.length>30?`...`:``}"`]}),(0,l.jsx)(`span`,{className:M.markerNote,children:e.comment})]})]},e.id)}),O&&!k&&Ft.filter(e=>!e.isFixed).map(e=>{let t=e.isMultiSelect;return(0,l.jsx)(`div`,{className:`${M.marker} ${M.hovered} ${t?M.multiSelect:``} ${M.exit}`,"data-annotation-marker":!0,style:{left:`${e.x}%`,top:e.y},children:(0,l.jsx)(ie,{size:t?12:10})},e.id)})]}),(0,l.jsxs)(`div`,{className:M.fixedMarkersLayer,"data-feedback-toolbar":!0,children:[O&&Pt.filter(e=>e.isFixed).map((e,t)=>{let n=Pt.filter(e=>e.isFixed),r=!k&&I===e.id,i=Me===e.id,a=r||i,o=e.isMultiSelect,s=o?`#34C759`:G.annotationColor,c=T.findIndex(t=>t.id===e.id),u=!rt.has(e.id),d=k?M.exit:Ae?M.clearing:u?M.enter:``;return(0,l.jsxs)(`div`,{className:`${M.marker} ${M.fixed} ${a?M.hovered:``} ${o?M.multiSelect:``} ${d}`,"data-annotation-marker":!0,style:{left:`${e.x}%`,top:e.y,backgroundColor:a?void 0:s,animationDelay:k?`${(n.length-1-t)*20}ms`:`${t*20}ms`},onMouseEnter:()=>!k&&e.id!==gt.current&&L(e.id),onMouseLeave:()=>L(null),onClick:t=>{t.stopPropagation(),k||Et(e.id)},onContextMenu:t=>{t.preventDefault(),t.stopPropagation(),k||Dt(e)},children:[a?(0,l.jsx)(m,{size:o?12:10}):(0,l.jsx)(`span`,{className:R!==null&&c>=R?M.renumber:void 0,children:c+1}),r&&!z&&(0,l.jsxs)(`div`,{className:`${M.markerTooltip} ${K?``:M.light} ${M.enter}`,style:It(e),children:[(0,l.jsxs)(`span`,{className:M.markerQuote,children:[e.element,e.selectedText&&` "${e.selectedText.slice(0,30)}${e.selectedText.length>30?`...`:``}"`]}),(0,l.jsx)(`span`,{className:M.markerNote,children:e.comment})]})]},e.id)}),O&&!k&&Ft.filter(e=>e.isFixed).map(e=>{let t=e.isMultiSelect;return(0,l.jsx)(`div`,{className:`${M.marker} ${M.fixed} ${M.hovered} ${t?M.multiSelect:``} ${M.exit}`,"data-annotation-marker":!0,style:{left:`${e.x}%`,top:e.y},children:(0,l.jsx)(m,{size:t?12:10})},e.id)})]}),f&&(0,l.jsxs)(`div`,{className:M.overlay,"data-feedback-toolbar":!0,style:P||z?{zIndex:99999}:void 0,children:[A?.rect&&!P&&!Le&&!X&&(0,l.jsx)(`div`,{className:`${M.hoverHighlight} ${M.enter}`,style:{left:A.rect.left,top:A.rect.top,width:A.rect.width,height:A.rect.height,borderColor:`${G.annotationColor}80`,backgroundColor:`${G.annotationColor}0A`}}),I&&!P&&(()=>{let e=T.find(e=>e.id===I);if(!e?.boundingBox)return null;let t=e.boundingBox,n=e.isMultiSelect;return(0,l.jsx)(`div`,{className:`${n?M.multiSelectOutline:M.singleSelectOutline} ${M.enter}`,style:{left:t.x,top:t.y-Fe,width:t.width,height:t.height,...n?{}:{borderColor:`${G.annotationColor}99`,backgroundColor:`${G.annotationColor}0D`}}})})(),A&&!P&&!Le&&!X&&(0,l.jsx)(`div`,{className:`${M.hoverTooltip} ${M.enter}`,style:{left:Math.max(8,Math.min(we.x,window.innerWidth-100)),top:Math.max(we.y-32,8)},children:A.element}),P&&(0,l.jsxs)(l.Fragment,{children:[P.boundingBox&&(0,l.jsx)(`div`,{className:`${P.isMultiSelect?M.multiSelectOutline:M.singleSelectOutline} ${st?M.exit:M.enter}`,style:{left:P.boundingBox.x,top:P.boundingBox.y-Fe,width:P.boundingBox.width,height:P.boundingBox.height,...P.isMultiSelect?{}:{borderColor:`${G.annotationColor}99`,backgroundColor:`${G.annotationColor}0D`}}}),(0,l.jsx)(`div`,{className:`${M.marker} ${M.pending} ${P.isMultiSelect?M.multiSelect:``} ${st?M.exit:M.enter}`,style:{left:`${P.x}%`,top:P.clientY,backgroundColor:P.isMultiSelect?`#34C759`:G.annotationColor},children:(0,l.jsx)(h,{size:12})}),(0,l.jsx)(p,{ref:_t,element:P.element,selectedText:P.selectedText,computedStyles:P.computedStylesObj,placeholder:P.element===`Area selection`?`What should change in this area?`:P.isMultiSelect?`Feedback for this group of elements...`:`What should change?`,onSubmit:wt,onCancel:Tt,isExiting:st,lightMode:!K,accentColor:P.isMultiSelect?`#34C759`:G.annotationColor,style:{left:Math.max(160,Math.min(window.innerWidth-160,P.x/100*window.innerWidth)),...P.clientY>window.innerHeight-290?{bottom:window.innerHeight-P.clientY+20}:{top:P.clientY+20}}})]}),z&&(0,l.jsxs)(l.Fragment,{children:[z.boundingBox&&(0,l.jsx)(`div`,{className:`${z.isMultiSelect?M.multiSelectOutline:M.singleSelectOutline} ${M.enter}`,style:{left:z.boundingBox.x,top:z.boundingBox.y-Fe,width:z.boundingBox.width,height:z.boundingBox.height,...z.isMultiSelect?{}:{borderColor:`${G.annotationColor}99`,backgroundColor:`${G.annotationColor}0D`}}}),(0,l.jsx)(p,{ref:vt,element:z.element,selectedText:z.selectedText,computedStyles:fe(z.computedStyles),placeholder:`Edit your feedback...`,initialValue:z.comment,submitLabel:`Save`,onSubmit:Ot,onCancel:kt,isExiting:lt,lightMode:!K,accentColor:z.isMultiSelect?`#34C759`:G.annotationColor,style:(()=>{let e=z.isFixed?z.y:z.y-Fe;return{left:Math.max(160,Math.min(window.innerWidth-160,z.x/100*window.innerWidth)),...e>window.innerHeight-290?{bottom:window.innerHeight-e+20}:{top:e+20}}})()})]}),X&&(0,l.jsxs)(l.Fragment,{children:[(0,l.jsx)(`div`,{ref:ft,className:M.dragSelection}),(0,l.jsx)(`div`,{ref:pt,className:M.highlightsContainer})]})]})]}),document.body)}var Te=`feedback-toolbar-settings`,P=18;function F(e){return n(e)&&typeof e.x==`number`&&typeof e.y==`number`&&typeof e.width==`number`&&typeof e.height==`number`}function Ee(e){return n(e)&&typeof e.id==`string`?e.id:null}function De(e){return!n(e)||typeof e.id!=`string`||typeof e.element!=`string`?null:{id:e.id,element:e.element,boundingBox:F(e.boundingBox)?e.boundingBox:void 0,isFixed:typeof e.isFixed==`boolean`?e.isFixed:void 0}}function Oe(e){if(!Array.isArray(e)||e.length===0)return!1;let t=e[0];return n(t)&&typeof t.id==`string`&&typeof t.element==`string`&&typeof t.elementPath==`string`}function ke(e){let t=[],n=e=>{for(let n=0;n<localStorage.length;n++){let r=localStorage.key(n);if(!(!r||!e(r)))try{let e=JSON.parse(localStorage.getItem(r)??`null`);if(!Oe(e))continue;for(let n of e){let e=De(n);e&&t.push(e)}}catch{continue}}};return n(t=>t.includes(e)&&t.includes(`feedback`)),t.length>0||n(e=>e.includes(`feedback`)),t}function Ae(){try{let e=localStorage.getItem(Te);return e?JSON.parse(e):null}catch{return null}}function je(e){try{localStorage.setItem(Te,JSON.stringify(e))}catch{}}function I(){return Ae()?.blockInteractions===!0}function L(e){return!!(e.closest(`[data-feedback-toolbar]`)||e.closest(`[data-annotation-popup]`)||e.closest(`[data-annotation-marker]`)||e.closest(`[data-devbar]`))}function Me(e,t){for(let n of document.elementsFromPoint(e,t))if(n instanceof HTMLElement&&n!==document.body&&!L(n))return n;return null}function Ne(){return!!document.getElementById(`feedback-cursor-styles`)}var R=new Set([`Fragment`,`Suspense`,`StrictMode`,`Provider`,`Consumer`,`Context`,`Slot`,`Portal`]);function Pe(e){if(typeof e!=`object`||!e)return null;let t=Reflect.get(e,`_debugSource`),n=Reflect.get(e,`_debugOwner`),r=typeof n==`object`&&n&&Reflect.get(n,`_debugSource`)!==void 0?n:e,i=t??(typeof n==`object`&&n?Reflect.get(n,`_debugSource`):void 0);if(typeof i!=`object`||!i)return null;let a=Reflect.get(i,`fileName`),o=Reflect.get(i,`lineNumber`),s=Reflect.get(i,`columnNumber`);if(typeof a!=`string`||typeof o!=`number`)return null;let c=a.lastIndexOf(`/ui/`),l=c===-1?a:a.slice(c+1),u=typeof s==`number`?`${l}:${o}:${s}`:`${l}:${o}`,d=Reflect.get(r,`type`),f=typeof d==`object`&&d?Reflect.get(d,`displayName`):void 0,p=typeof d==`function`?d.name:typeof d==`object`&&d?Reflect.get(d,`name`):void 0,m=typeof f==`string`?f:typeof p==`string`&&p.length>0?p:null;return m?{name:m,location:u}:null}function z(e){if(!(e instanceof HTMLElement))return null;let t=Object.getOwnPropertyNames(e).find(e=>e.startsWith(`__reactFiber$`)||e.startsWith(`__reactInternalInstance$`));if(!t)return null;let n=Reflect.get(e,t),r=[];for(let e=0;typeof n==`object`&&n&&e<100;e++){let e=Pe(n);if(e&&!R.has(e.name)&&!r.some(t=>t.name===e.name&&t.location===e.location)&&(r.push(e),r.length>=5))break;n=Reflect.get(n,`return`)}if(r.length===0)return null;let i=r[0];return i?r.length===1?`${i.name} (${i.location})`:`${r.slice().reverse().map(e=>e.name).join(` > `)} (${i.location})`:null}function B(e){let t=e.boundingBox;if(!t)return null;let n=t.x+t.width/2,r=t.y+t.height/2,i=Me(n,e.isFixed?r:r-window.scrollY);return i?z(i):null}function Fe(e,t){let n=e.split(`
`);for(let e=0;e<n.length;e++){let r=n[e];if(!r)continue;let i=/^###\s+(\d+)\.\s/.exec(r);if(!i?.[1])continue;let a=Number(i[1]),o=t.get(a);if(o)for(let t=e+1;t<Math.min(e+20,n.length);t++){let e=n[t];if(e){if(e.startsWith(`### `))break;if(e.startsWith(`**Location:**`)){n[t]=`**Location:** ${o}`;break}}}}return n.join(`
`)}function Ie(){let e=(0,o.c)(1),t;e[0]===Symbol.for(`react.memo_cache_sentinel`)?(t=[],e[0]=t):t=e[0],(0,s.useEffect)(Le,t)}function Le(){let e=Ae();e?.blockInteractions===!1?je({...e,blockInteractions:!0}):e||je({blockInteractions:!0})}function Re(){let e=(0,o.c)(1),t;e[0]===Symbol.for(`react.memo_cache_sentinel`)?(t=[],e[0]=t):t=e[0],(0,s.useEffect)(V,t)}function V(){let e=ze;return document.addEventListener(`click`,e,!0),()=>document.removeEventListener(`click`,e,!0)}function ze(e){if(!Ne()||!I())return;let t=e.target;t instanceof HTMLElement&&(L(t)||(e.preventDefault(),e.stopPropagation()))}function H(){let e=(0,o.c)(1),t;e[0]===Symbol.for(`react.memo_cache_sentinel`)?(t=[],e[0]=t):t=e[0],(0,s.useEffect)(Be,t)}function Be(){let e=document.getElementById(`agentation-devbar-offset`),t=e instanceof HTMLStyleElement?e:document.head.appendChild(Object.assign(document.createElement(`style`),{id:`agentation-devbar-offset`})),n=()=>{let e=!!document.querySelector(`[data-devbar]`);t.textContent=e?`[data-feedback-toolbar] { bottom: calc(1.25rem + ${P}px) !important; }`:``};n();let r=new MutationObserver(n);return r.observe(document.body,{childList:!0,subtree:!0}),()=>{r.disconnect(),t.textContent=``}}function U(){let e=(0,o.c)(9),t;e[0]===Symbol.for(`react.memo_cache_sentinel`)?(t=[],e[0]=t):t=e[0];let n=(0,s.useRef)(t),r;e[1]===Symbol.for(`react.memo_cache_sentinel`)?(r=new Map,e[1]=r):r=e[1];let i=(0,s.useRef)(r),a=(0,s.useRef)(null);Ie(),H(),Re();let c,u;e[2]===Symbol.for(`react.memo_cache_sentinel`)?(c=()=>{let e=e=>{let t=e.target;if(!(t instanceof HTMLElement)||L(t))return;let n=Me(e.clientX,e.clientY);a.current=n?z(n):null};return document.addEventListener(`click`,e,!0),()=>document.removeEventListener(`click`,e,!0)},u=[],e[2]=c,e[3]=u):(c=e[2],u=e[3]),(0,s.useEffect)(c,u);let d;e[4]===Symbol.for(`react.memo_cache_sentinel`)?(d=e=>{let t=De(e);if(!t)return;n.current=[...n.current,t];let r=a.current;if(r)i.current.set(t.id,r);else{let e=B(t);e&&i.current.set(t.id,e)}a.current=null},e[4]=d):d=e[4];let f=d,p;e[5]===Symbol.for(`react.memo_cache_sentinel`)?(p=e=>{let t=Ee(e);t&&(n.current=n.current.filter(e=>e.id!==t),i.current.delete(t))},e[5]=p):p=e[5];let m=p,h;e[6]===Symbol.for(`react.memo_cache_sentinel`)?(h=()=>{n.current=[],i.current.clear(),a.current=null},e[6]=h):h=e[6];let ee=h,g;e[7]===Symbol.for(`react.memo_cache_sentinel`)?(g=async e=>{let t=ke(window.location.pathname),r=t.length>0?t:n.current;if(r.length===0){try{await navigator.clipboard.writeText(e)}catch{}return}let a=new Map;r.forEach((e,t)=>{let n=i.current.get(e.id),r=n??B(e);r&&(n||i.current.set(e.id,r),a.set(t+1,r))});try{await navigator.clipboard.writeText(Fe(e,a))}catch{}},e[7]=g):g=e[7];let te=g,_;return e[8]===Symbol.for(`react.memo_cache_sentinel`)?(_=(0,l.jsx)(we,{copyToClipboard:!1,onCopy:te,onAnnotationAdd:f,onAnnotationDelete:m,onAnnotationsClear:ee}),e[8]=_):_=e[8],_}export{U as default};
//# sourceMappingURL=AgentationWrapper-CDCC5fIO.js.map