// Owned simple-line command lowering, not a grammar service, decoder or public ABI.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {logicalLines,byteToLine} from './reference-profile.mjs';
import {prepareSelectionEdit,prepareDerivedSelectionEdit,publishSelectionEdit,selectionInvocationIntent} from './selection-products.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./line-edits.json',import.meta.url),'utf8'));
const selection=JSON.parse(fs.readFileSync(new URL('./selection-products.json',import.meta.url),'utf8'));
const owned=new WeakMap(),copy=structuredClone;
export function lineInputForm(){return Object.freeze({family:catalog.family,contract:catalog.contract,operation:catalog.variant,variant:catalog.variant,profileConstant:catalog.profile,fields:Object.freeze([...catalog.common])});}
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const validLimits=l=>l&&catalog.limits.every(k=>Number.isSafeInteger(l[k])&&l[k]>=0);
const prefixEnd=(source,start,end)=>{let at=start;while(at<end&&(source[at]===32||source[at]===9))at++;return at;};
function column(source,start,end,width){let col=0;for(let i=start;i<end;i++)col+=source[i]===9?width-col%width:1;return col;}
function settingsValid(r,l){
  if(r.action==='insert-newline')return exact(r.settings,catalog.newlineSettings)&&Object.hasOwn(catalog.delimiters,r.settings.delimiter)&&catalog.autoIndent.includes(r.settings.autoIndent);
  const s=r.settings;
  return exact(s,catalog.indentSettings)&&typeof s.unit==='string'&&(s.unit==='\t'||/^ +$/.test(s.unit))&&s.unit.length<=l.maxUnitBytes&&
    Number.isSafeInteger(s.tabWidth)&&s.tabWidth>0&&s.tabWidth<=l.maxIndentColumns;
}
export function prepareLineEdit(state,request,g,limits,workLimits){
  if(!Array.isArray(state?.views)||!Array.isArray(state?.documents)||!validLimits(workLimits)||!exact(request,catalog.common)||request.contract!==catalog.contract||request.operation!==catalog.variant||
      !catalog.actions.includes(request.action)||request.profile!==catalog.profile||!settingsValid(request,workLimits)||!equal(request.settings,g.lineSettings))return null;
  const common={contract:selection.contract,operation:'ordinary-edit',invocation:request.invocation,view:request.view,document:request.document,
    sourceRevision:request.sourceRevision,expectedSelection:request.expectedSelection,profile:selection.profile,text:'',selectionAfter:request.action==='insert-newline'?'carets-after':'mapped'};
  // Validate the exact target/current normalized selections before derived work.
  const probe={...common,operation:'logical-movement',unit:'document-start',extend:false};delete probe.text;delete probe.selectionAfter;
  if(!prepareSelectionEdit(state,probe,g,limits))return null;
  const d=state.documents.find(d=>equal(d.document,request.document)),v=state.views.find(v=>equal(v.view,request.view));
  const source=d.source,lines=logicalLines(source),edits=[],selected=new Set();let prefixBytes=0,maxColumns=0;
  for(const s of v.selections){
    const lo=Math.min(s.anchor,s.head),hi=Math.max(s.anchor,s.head),from=byteToLine(source,lo),to=byteToLine(source,hi);
    if(!from||!to)return null; // CRLF interior is unavailable, never rounded.
    if(request.action==='insert-newline'){
      const row=lines[from.line],end=request.settings.autoIndent==='copy-prefix'?prefixEnd(source,row.start,Math.min(row.bodyEnd,lo)):row.start;
      const prefix=request.settings.autoIndent==='copy-prefix'?source.slice(row.start,end):[];
      prefixBytes+=prefix.length;if(prefix.length>workLimits.maxPrefixBytes||prefixBytes>workLimits.maxPrefixBytes)return null;
      edits.push({lo,hi,expected:source.slice(lo,hi),ins:[...catalog.delimiters[request.settings.delimiter],...prefix]});
    }else{
      // Half-open nonempty source selection excludes a line starting at its upper end.
      const last=hi>lo&&hi===lines[to.line].start?to.line-1:to.line;
      for(let line=from.line;line<=last;line++){selected.add(line);if(selected.size>workLimits.maxLines)return null;}
    }
  }
  if(request.action==='insert-newline'&&v.selections.length>workLimits.maxLines)return null;
  if(request.action!=='insert-newline')for(const index of [...selected].sort((a,b)=>a-b)){
    const row=lines[index],end=prefixEnd(source,row.start,row.bodyEnd),size=end-row.start;
    prefixBytes+=size;if(prefixBytes>workLimits.maxPrefixBytes)return null;
    const s=request.settings,unit=Array.from(Buffer.from(s.unit,'utf8')),current=column(source,row.start,end,s.tabWidth),step=s.unit==='\t'?s.tabWidth:s.unit.length;
    const target=request.action==='indent-lines'?column([...unit,...source.slice(row.start,end)],0,unit.length+size,s.tabWidth):Math.max(0,current-step);
    maxColumns=Math.max(maxColumns,current,target);if(maxColumns>workLimits.maxIndentColumns)return null;
    if(request.action==='indent-lines')edits.push({lo:row.start,hi:row.start,expected:[],ins:unit});
    else{
      const replacement=s.unit==='\t'?[...Array(Math.floor(target/s.tabWidth)).fill(9),...Array(target%s.tabWidth).fill(32)]:Array(target).fill(32);
      // Retain every shared leading byte; do not replace untouched whitespace.
      let keep=0;while(keep<size&&keep<replacement.length&&source[row.start+keep]===replacement[keep])keep++;
      if(keep<size||keep<replacement.length)edits.push({lo:row.start+keep,hi:end,expected:source.slice(row.start+keep,end),ins:replacement.slice(keep)});
    }
  }
  const charges={lines:request.action==='insert-newline'?v.selections.length:selected.size,prefixBytes,maxColumns,unitBytes:request.settings.unit?.length??0,
    derivedBytes:Buffer.byteLength(JSON.stringify({request,edits}),'utf8')};
  if(charges.derivedBytes>workLimits.maxDerivedBytes)return null;
  const core=prepareDerivedSelectionEdit(state,common,edits,g,limits);if(!core)return null;
  const plan=Object.freeze({contract:catalog.contract,action:request.action});
  owned.set(plan,{request:copy(request),core,settings:copy(request.settings),limits:copy(limits),charges});return plan;
}
export function lineInvocationIntent(plan,maximum){
  const p=owned.get(plan);if(!p||!Number.isSafeInteger(maximum)||maximum<0)return null;
  const core=selectionInvocationIntent(p.core,maximum);if(!core)return null;
  const value={request:copy(p.request),settings:copy(p.settings),limits:copy(p.limits),charges:copy(p.charges),core:core.value};
  return Buffer.byteLength(JSON.stringify(value),'utf8')<=maximum?{invocation:p.request.invocation,value}:null;
}
// The private core handle is reachable only through this retained line record.
export function discardLineEdit(plan,g){return ['ownerControlAllowed','retentionReleased','deliveryDrained'].every(k=>g?.[k]===true)&&owned.delete(plan);}
export function publishLineEdit(state,plan,g,limits,workLimits){
  const p=owned.get(plan),refuse=()=>({status:'not-published',state});
  if(!p||!Array.isArray(state?.views)||!Array.isArray(state?.documents)||!validLimits(workLimits)||!equal(p.settings,g.lineSettings)||!equal(p.limits,limits)||p.charges.lines>workLimits.maxLines||
      p.charges.prefixBytes>workLimits.maxPrefixBytes||p.charges.maxColumns>workLimits.maxIndentColumns||p.charges.unitBytes>workLimits.maxUnitBytes||p.charges.derivedBytes>workLimits.maxDerivedBytes)return refuse();
  return publishSelectionEdit(state,p.core,g);
}
