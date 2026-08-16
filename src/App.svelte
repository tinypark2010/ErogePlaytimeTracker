<script lang="ts">
 import {onMount} from 'svelte'; import {listen} from '@tauri-apps/api/event'; import Library from './components/Library.svelte'; import GameDetail from './components/GameDetail.svelte'; import AddGame from './components/AddGame.svelte'; import Settings from './components/Settings.svelte'; import {api} from './lib/api'; import type {Theme,TrackingStatus} from './lib/types';
 let page:'library'|'game'|'add'|'settings'='library', gameId=0, refresh=0, status:TrackingStatus={running_games:[],foreground_game_id:null},pendingBackground:TrackingStatus|null=null,backgroundTimer:number|undefined;
 const openGame=(id:number)=>{gameId=id;page='game'}; const reload=()=>refresh++;
 $: isRunning=status.running_games.length>0;
 $: isForeground=isRunning&&status.foreground_game_id!==null;
 $: trackingText=!isRunning?'待機中':isForeground?`プレイ時間を記録中（${status.running_games.find(g=>g.game_id===status.foreground_game_id)?.title??'ゲーム'}）`:`${status.running_games.length}本起動中・バックグラウンド時間は除外中`;
 const applyTheme=(theme:Theme)=>document.documentElement.dataset.theme=theme;
 function updateStatus(next:TrackingStatus){const background=next.running_games.length>0&&next.foreground_game_id===null;const alreadyBackground=status.running_games.length>0&&status.foreground_game_id===null;if(background&&!alreadyBackground){pendingBackground=next;if(backgroundTimer===undefined)backgroundTimer=window.setTimeout(()=>{if(pendingBackground)status=pendingBackground;pendingBackground=null;backgroundTimer=undefined},3500);return}if(backgroundTimer!==undefined){clearTimeout(backgroundTimer);backgroundTimer=undefined}pendingBackground=null;status=next}
 onMount(()=>{api.settings().then(v=>applyTheme(v.theme));api.status().then(updateStatus);const timer=setInterval(()=>api.status().then(updateStatus),3000);let off=()=>{};listen<TrackingStatus>('tracking-status',e=>updateStatus(e.payload)).then(f=>off=f);return()=>{clearInterval(timer);if(backgroundTimer!==undefined)clearTimeout(backgroundTimer);off()}});
</script>
<header><button class="brand" onclick={()=>{page='library';reload()}}>Eroge Playtime Tracker</button><nav><button onclick={()=>page='library'}>ライブラリ</button><button onclick={()=>page='add'}>ゲーム追加</button><button onclick={()=>page='settings'}>設定</button></nav><div class="tracking-status" class:active={isForeground} class:background={isRunning&&!isForeground}>● {trackingText}</div></header>
<main>
 {#if page==='library'}<Library {refresh} {openGame}/>{:else if page==='game'}<GameDetail {gameId} onback={()=>{page='library';reload()}}/>{:else if page==='add'}<AddGame ondone={(id)=>{openGame(id)}} oncancel={()=>page='library'}/>{:else}<Settings ontheme={applyTheme}/>{/if}
</main>
