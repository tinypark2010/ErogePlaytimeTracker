import { convertFileSrc } from '@tauri-apps/api/core';

export function duration(seconds:number|null){
  if(seconds===null)return '実行中';
  const total=Math.max(0,Math.floor(seconds));
  const h=Math.floor(total/3600),m=Math.floor(total%3600/60),s=total%60;
  if(h)return `${h}時間 ${m}分 ${s}秒`;
  if(m)return `${m}分 ${s}秒`;
  return `${s}秒`;
}
export function local(iso:string|null){return iso?new Date(iso).toLocaleString():'実行中'}
export function lastPlayed(iso:string|null){return iso?new Date(iso).toLocaleString():'未プレイ'}
export function inputTime(iso:string|null){if(!iso)return '';const d=new Date(iso);const local=new Date(d.getTime()-d.getTimezoneOffset()*60000);return local.toISOString().slice(0,19)}
export function utc(value:string){return new Date(value).toISOString()}
export function imageSrc(path:string|null){if(!path)return '';return (window as unknown as {__TAURI_INTERNALS__?:unknown}).__TAURI_INTERNALS__?convertFileSrc(path):path}
