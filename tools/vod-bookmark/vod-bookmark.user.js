// ==UserScript==
// @name         다시보기 책갈피 (숲·치지직)
// @namespace    https://github.com/sinwa3/pc-toolbox
// @version      0.1.1
// @author       sinwa
// @description  숲·치지직 다시보기에 시간 책갈피(메모 포함, 영상당 5개)를 남기고, 어느 화면에서든 책갈피한 영상 목록을 열어 그 시간으로 바로 간다.
// @match        https://chzzk.naver.com/*
// @match        https://*.sooplive.com/*
// @match        https://*.sooplive.co.kr/*
// @noframes
// @grant        GM_getValue
// @grant        GM_setValue
// @grant        GM_addValueChangeListener
// @updateURL    https://raw.githubusercontent.com/sinwa3/pc-toolbox/main/tools/vod-bookmark/vod-bookmark.user.js
// @downloadURL  https://raw.githubusercontent.com/sinwa3/pc-toolbox/main/tools/vod-bookmark/vod-bookmark.user.js
// ==/UserScript==

(() => {
  'use strict';

  const MAX_MARKS = 5;          // 영상당 책갈피 상한. 가득 차면 추가를 막는다
  const STORE_KEY = 'videos';   // { "<사이트>:<번호>": { site, id, url, title, channel, updatedAt, marks: [{ sec, memo }] } }
  const BM_WAIT_MS = 30000;     // #bm= 이동 시 플레이어 준비를 기다리는 최대 시간

  // ---------- 공통 ----------

  const toSeconds = (time) => {
    const parts = time.split(':').map(Number);
    if (parts.some(Number.isNaN) || parts.slice(1).some(n => n >= 60)) return null;
    return parts.reduce((acc, n) => acc * 60 + n, 0);
  };

  const fmt = (sec) => {
    sec = Math.floor(sec);
    const pad = n => String(n).padStart(2, '0');
    const h = Math.floor(sec / 3600);
    const m = Math.floor(sec % 3600 / 60);
    return h ? `${h}:${pad(m)}:${pad(sec % 60)}` : `${m}:${pad(sec % 60)}`;
  };

  const getJson = async (url, init = {}) => {
    const res = await fetch(url, { credentials: 'include', ...init });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return res.json();
  };

  // ---------- 사이트별 ----------

  const SITES = {
    soop: {
      label: '숲',
      host: /(^|\.)sooplive\.(com|co\.kr)$/,
      videoId: () => location.hostname.startsWith('vod.') ? (location.pathname.match(/^\/player\/(\d+)/) || [])[1] : undefined,
      url: (id) => `${location.origin}/player/${id}`,
      info: async (id) => {
        const root = location.hostname.replace(/^vod\./, '');
        const d = (await getJson(`https://api.m.${root}/station/video/a/view`, { method: 'POST', body: new URLSearchParams({ nTitleNo: id }) })).data || {};
        return { title: d.full_title || d.title_name || d.title, channel: d.writer_nick || d.bj_nick || d.bj_id };
      },
      // 긴 다시보기는 여러 파일로 나뉘어 video.currentTime이 전체 시간과 안 맞는다. 재생바 글자를 읽는다
      currentSec: () => {
        const t = document.querySelector('.time-current');
        return t ? toSeconds(t.textContent.trim()) ?? 0 : 0;
      },
      // 같은 이유로 숲의 댓글 시간 링크(a.time_link[data-time]) 위임 클릭 처리기를 빌려 쓴다 (soop-timeline과 같은 방식)
      seek: (sec) => {
        const a = document.createElement('a');
        a.className = 'time_link';
        a.dataset.time = String(sec);
        a.style.display = 'none';
        document.body.appendChild(a);
        a.click();
        a.remove();
      },
      ready: () => !!document.querySelector('.time-current') && document.querySelector('#player video')?.readyState >= 1,
      buttonHost: () => document.querySelector('.right_ctrl'),
      panelHost: () => document.getElementById('player'),
    },
    chzzk: {
      label: '치지직',
      host: /^chzzk\.naver\.com$/,
      videoId: () => (location.pathname.match(/^\/video\/(\d+)/) || [])[1],
      url: (id) => `https://chzzk.naver.com/video/${id}`,
      info: async (id) => {
        const c = (await getJson(`https://api.chzzk.naver.com/service/v2/videos/${id}`)).content || {};
        return { title: c.videoTitle?.trim(), channel: c.channel?.channelName };
      },
      // 본 영상은 파일 하나라 currentTime이 전체 시간과 같다. 나머지 <video>는 광고용
      video: () => document.querySelector('video.webplayer-internal-video'),
      currentSec: () => SITES.chzzk.video()?.currentTime || 0,
      seek: (sec) => {
        const v = SITES.chzzk.video();
        if (v) v.currentTime = sec;
      },
      ready: () => {
        const v = SITES.chzzk.video();
        return !!v && v.readyState >= 1 && v.duration > 0;
      },
      buttonHost: () => document.querySelector('.pzp-pc__bottom-buttons-right'),
      panelHost: () => document.querySelector('.pzp-pc'),
    },
  };

  const siteKey = Object.keys(SITES).find(k => SITES[k].host.test(location.hostname));
  if (!siteKey) return;
  const site = SITES[siteKey];

  const keyOf = (s, id) => `${s}:${id}`;
  const current = () => {
    const id = site.videoId();
    return id ? { id, key: keyOf(siteKey, id) } : null;
  };

  // ---------- 저장 ----------

  const loadAll = () => GM_getValue(STORE_KEY, {});
  const saveAll = (all) => {
    GM_setValue(STORE_KEY, all);
    renderAll();
  };

  const infoCache = new Map();
  const getInfo = async (cur) => {
    if (!infoCache.has(cur.key)) {
      let info = {};
      try { info = await site.info(cur.id); } catch { /* 제목은 문서 제목으로 대신한다 */ }
      infoCache.set(cur.key, info);
    }
    return infoCache.get(cur.key);
  };

  const addMark = async (memo) => {
    const cur = current();
    if (!cur) return;
    const sec = Math.floor(site.currentSec());
    const info = await getInfo(cur);
    const all = loadAll();
    const rec = all[cur.key] || { site: siteKey, id: cur.id, url: site.url(cur.id), marks: [] };
    if (rec.marks.length >= MAX_MARKS) return setStatus(`책갈피는 영상당 ${MAX_MARKS}개까지예요. 하나를 지우고 추가하세요`);
    if (rec.marks.some(m => m.sec === sec)) return setStatus(`${fmt(sec)}는 이미 있어요`);
    rec.title = info.title || rec.title || document.title;
    rec.channel = info.channel || rec.channel || '';
    rec.marks.push({ sec, memo });
    rec.marks.sort((a, b) => a.sec - b.sec);
    rec.updatedAt = Date.now();
    all[cur.key] = rec;
    saveAll(all);
    memoInput.value = '';
    setStatus(`${fmt(sec)} 추가함`);
  };

  // 지울 때는 순서(updatedAt)를 바꾸지 않는다. 마지막 책갈피를 지우면 영상도 목록에서 뺀다
  const removeMark = (key, sec) => {
    const all = loadAll();
    const rec = all[key];
    if (!rec) return;
    rec.marks = rec.marks.filter(m => m.sec !== sec);
    if (!rec.marks.length) delete all[key];
    saveAll(all);
  };

  // ---------- 이동 ----------

  // 플레이어가 준비 직후 이어보기 위치로 다시 옮기는 경우가 있어, 잠시 뒤 한 번 더 확인한다
  const seekSure = (sec) => {
    site.seek(sec);
    setTimeout(() => {
      if (Math.abs(site.currentSec() - sec) > 3) site.seek(sec);
    }, 1500);
  };

  const openVideo = (rec, sec) => {
    const cur = current();
    if (cur && cur.key === keyOf(rec.site, rec.id)) {
      seekSure(sec);
      setListOpen(false);
      return;
    }
    location.href = sec ? `${rec.url}#bm=${sec}` : rec.url;
  };

  let bmTimer = null;
  const handleBm = () => {
    clearInterval(bmTimer);
    const m = location.hash.match(/bm=(\d+)/);
    if (!m || !current()) return;
    const sec = Number(m[1]);
    const started = Date.now();
    bmTimer = setInterval(() => {
      if (Date.now() - started > BM_WAIT_MS) return clearInterval(bmTimer);
      if (!site.ready()) return;
      clearInterval(bmTimer);
      seekSure(sec);
      // 새로고침할 때마다 다시 옮기지 않게 주소에서 지운다
      history.replaceState(history.state, '', location.pathname + location.search);
    }, 300);
  };

  // ---------- 화면 ----------

  const style = document.createElement('style');
  style.textContent = `
    button.vbm-btn {
      width: 36px; height: 36px; margin-right: 2px; padding: 0; align-self: center;
      display: flex; align-items: center; justify-content: center;
      background: none; border: 0; color: #fff; cursor: pointer; opacity: .85;
    }
    button.vbm-btn:hover, button.vbm-btn.on { opacity: 1; color: #ffc94d; }
    #vbm-panel {
      position: absolute; top: 0; left: 0; z-index: 1000;
      width: min(320px, 45%); max-height: calc(100% - 72px); display: none; flex-direction: column;
      background: rgba(16, 16, 16, .78); backdrop-filter: blur(6px);
      color: #ddd; font: 14px/1.5 sans-serif; border-bottom-right-radius: 6px;
    }
    #vbm-panel.open { display: flex; }
    #vbm-panel .vbm-head { display: flex; gap: 6px; align-items: center; padding: 8px 8px 6px; }
    #vbm-panel .vbm-head input {
      flex: 1; min-width: 0; height: 28px; padding: 0 8px; color: #eee; font-size: 13px; outline: none;
      background: rgba(255,255,255,.08); border: 1px solid rgba(255,255,255,.18); border-radius: 4px;
    }
    #vbm-panel .vbm-head input:focus { border-color: #ffc94d; }
    #vbm-panel .vbm-add {
      flex: none; height: 28px; padding: 0 10px; cursor: pointer; font-size: 13px; font-weight: bold;
      color: #1a1a1a; background: #ffc94d; border: 0; border-radius: 4px;
    }
    #vbm-panel .vbm-add:disabled { cursor: default; opacity: .4; }
    #vbm-panel .vbm-icon {
      flex: none; width: 28px; height: 28px; padding: 0; cursor: pointer; color: #ccc;
      background: none; border: 1px solid rgba(255,255,255,.18); border-radius: 4px;
    }
    #vbm-panel .vbm-icon:hover { color: #fff; border-color: #fff; }
    #vbm-panel .vbm-status { padding: 0 10px 6px; font-size: 12px; color: #aaa; }
    #vbm-panel .vbm-status:empty { display: none; }
    #vbm-panel .vbm-marks { overflow-y: auto; padding: 0 4px 8px; overscroll-behavior: contain; }
    #vbm-panel .vbm-row, #vbm-list .vbm-row { display: flex; gap: 8px; align-items: center; padding: 4px 8px; border-radius: 3px; cursor: pointer; }
    #vbm-panel .vbm-row:hover, #vbm-list .vbm-row:hover { background: rgba(255,255,255,.08); }
    #vbm-panel .vbm-time, #vbm-list .vbm-time { flex: none; color: #ffc94d; font-variant-numeric: tabular-nums; font-weight: bold; }
    #vbm-panel .vbm-memo, #vbm-list .vbm-memo { flex: 1; min-width: 0; word-break: break-all; }
    #vbm-panel .vbm-del, #vbm-list .vbm-del { flex: none; padding: 0 4px; color: #777; background: none; border: 0; cursor: pointer; font-size: 13px; }
    #vbm-panel .vbm-del:hover, #vbm-list .vbm-del:hover { color: #ff6b6b; }
    #vbm-panel .vbm-empty, #vbm-list .vbm-empty { padding: 4px 10px 10px; color: #888; font-size: 13px; }

    #vbm-fab {
      position: fixed; left: 12px; bottom: 12px; z-index: 2147483000;
      width: 36px; height: 36px; padding: 0; display: flex; align-items: center; justify-content: center;
      color: #ffc94d; background: rgba(20, 20, 20, .8); border: 1px solid rgba(255,255,255,.2); border-radius: 50%;
      cursor: pointer; opacity: .55; box-shadow: 0 2px 6px rgba(0,0,0,.3);
    }
    #vbm-fab:hover, #vbm-fab.on { opacity: 1; }
    #vbm-list {
      position: fixed; left: 12px; bottom: 56px; z-index: 2147483000;
      width: min(380px, calc(100vw - 24px)); max-height: 65vh; display: none; flex-direction: column;
      background: rgba(20, 20, 20, .95); color: #ddd; font: 14px/1.5 sans-serif; text-align: left;
      border: 1px solid rgba(255,255,255,.15); border-radius: 8px; box-shadow: 0 6px 20px rgba(0,0,0,.4);
    }
    #vbm-list.open { display: flex; }
    #vbm-list .vbm-list-head { padding: 10px 12px 6px; font-weight: bold; color: #fff; }
    #vbm-list .vbm-videos { overflow-y: auto; padding: 0 6px 8px; overscroll-behavior: contain; }
    #vbm-list .vbm-video { border-top: 1px solid rgba(255,255,255,.08); padding: 4px 0; }
    #vbm-list .vbm-vhead { display: flex; gap: 6px; align-items: flex-start; padding: 4px 6px; }
    #vbm-list .vbm-badge { flex: none; padding: 0 5px; border-radius: 3px; font-size: 11px; line-height: 18px; margin-top: 1px; color: #111; }
    #vbm-list .vbm-badge.soop { background: #5aa9ff; }
    #vbm-list .vbm-badge.chzzk { background: #00ffa3; }
    #vbm-list .vbm-vinfo { flex: 1; min-width: 0; }
    #vbm-list .vbm-title { display: block; color: #eee; cursor: pointer; word-break: break-all; }
    #vbm-list .vbm-title:hover { color: #ffc94d; text-decoration: underline; }
    #vbm-list .vbm-sub { font-size: 12px; color: #888; }
    #vbm-list .vbm-toggle { flex: none; width: 24px; height: 24px; padding: 0; color: #aaa; background: none; border: 0; cursor: pointer; }
    #vbm-list .vbm-toggle:hover { color: #fff; }
    #vbm-list .vbm-video .vbm-row { margin-left: 26px; }
  `;
  document.head.appendChild(style);

  const el = (tag, props = {}, children = []) => {
    const node = Object.assign(document.createElement(tag), props);
    node.append(...children);
    return node;
  };

  const ICON = '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round">' +
    '<path d="M6 3h12v18l-6-4.5L6 21z"/></svg>';

  const markRow = (key, m, onClick) => {
    const del = el('button', { type: 'button', className: 'vbm-del', title: '삭제', textContent: '✕' });
    del.addEventListener('click', (e) => { e.stopPropagation(); removeMark(key, m.sec); });
    const row = el('div', { className: 'vbm-row' }, [
      el('span', { className: 'vbm-time', textContent: fmt(m.sec) }),
      el('span', { className: 'vbm-memo', textContent: m.memo }),
      del,
    ]);
    row.addEventListener('click', onClick);
    return row;
  };

  // --- 다시보기 화면의 패널 ---

  const memoInput = el('input', { type: 'text', placeholder: '메모 (Enter로 추가)', maxLength: 100 });
  const addBtn = el('button', { type: 'button', className: 'vbm-add', title: '지금 시간을 책갈피로', textContent: '+ 지금' });
  const closeBtn = el('button', { type: 'button', className: 'vbm-icon', title: '닫기 (B)', textContent: '✕' });
  const status = el('div', { className: 'vbm-status' });
  const marksBox = el('div', { className: 'vbm-marks' });
  const panel = el('div', { id: 'vbm-panel' }, [
    el('div', { className: 'vbm-head' }, [memoInput, addBtn, closeBtn]),
    status,
    marksBox,
  ]);

  // 패널 안의 조작이 플레이어(클릭 재생/정지, 휠 볼륨, 단축키)로 새지 않게 막는다
  for (const type of ['click', 'dblclick', 'mousedown', 'mouseup', 'pointerdown', 'pointerup', 'wheel', 'keydown', 'keyup', 'keypress']) {
    panel.addEventListener(type, e => e.stopPropagation());
  }

  let statusTimer = null;
  const setStatus = (text) => {
    status.textContent = text;
    clearTimeout(statusTimer);
    statusTimer = setTimeout(() => { status.textContent = ''; }, 3000);
  };

  const renderPanel = () => {
    const cur = current();
    const rec = cur && loadAll()[cur.key];
    const marks = rec?.marks || [];
    addBtn.disabled = marks.length >= MAX_MARKS;
    addBtn.title = addBtn.disabled ? `영상당 ${MAX_MARKS}개까지예요` : '지금 시간을 책갈피로';
    marksBox.replaceChildren(...marks.map(m => markRow(cur.key, m, () => seekSure(m.sec))));
    if (!marks.length) marksBox.append(el('div', { className: 'vbm-empty', textContent: `지금 보는 시간을 책갈피로 남겨요 (영상당 ${MAX_MARKS}개)` }));
  };

  // 평소엔 플레이어 안에 붙인다. 전체화면 요소가 플레이어를 품고 있지 않으면 그쪽으로 옮겨야 보인다.
  const attachPanel = () => {
    const player = site.panelHost();
    const fs = document.fullscreenElement;
    const host = fs && fs.tagName !== 'VIDEO' && !(player && fs.contains(player)) ? fs : player;
    if (!host || panel.parentElement === host) return;
    if (getComputedStyle(host).position === 'static') host.style.position = 'relative';
    host.appendChild(panel);
  };
  document.addEventListener('fullscreenchange', attachPanel);

  const setPanelOpen = (open) => {
    if (open && !current()) return;
    if (open) {
      attachPanel();
      renderPanel();
    }
    panel.classList.toggle('open', open);
    playerBtn.classList.toggle('on', open);
  };

  addBtn.addEventListener('click', () => addMark(memoInput.value.trim()));
  memoInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.isComposing) addMark(memoInput.value.trim());
    if (e.key === 'Escape') memoInput.blur();
  });
  closeBtn.addEventListener('click', () => setPanelOpen(false));

  // 재생바 오른쪽 버튼 묶음 맨 앞에 버튼을 붙인다. 플레이어가 다시 그려지면 다시 붙인다.
  const playerBtn = el('button', { type: 'button', className: 'vbm-btn', title: '책갈피 (B)', innerHTML: ICON });
  playerBtn.addEventListener('click', (e) => { e.stopPropagation(); setPanelOpen(!panel.classList.contains('open')); });

  let pending = false;
  new MutationObserver(() => {
    if (pending) return;
    pending = true;
    requestAnimationFrame(() => {
      pending = false;
      if (!current()) return;
      const ctrl = site.buttonHost();
      if (ctrl && !ctrl.contains(playerBtn)) ctrl.prepend(playerBtn);
    });
  }).observe(document.body, { childList: true, subtree: true });

  document.addEventListener('keydown', (e) => {
    if (e.code !== 'KeyB' || e.ctrlKey || e.altKey || e.metaKey || e.repeat || !current()) return;
    const t = e.target;
    if (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName)) return;
    e.preventDefault();
    setPanelOpen(!panel.classList.contains('open'));
  }, true);

  // --- 모든 화면의 "책갈피 영상" 버튼과 목록 ---

  const expanded = new Set();
  const videosBox = el('div', { className: 'vbm-videos' });
  const listHead = el('div', { className: 'vbm-list-head' });
  const list = el('div', { id: 'vbm-list' }, [listHead, videosBox]);
  const fab = el('button', { type: 'button', id: 'vbm-fab', title: '책갈피 영상', innerHTML: ICON });
  document.body.append(list, fab);

  const renderList = () => {
    if (!list.classList.contains('open')) return;
    const recs = Object.entries(loadAll()).sort((a, b) => (b[1].updatedAt || 0) - (a[1].updatedAt || 0));
    listHead.textContent = `책갈피 영상 · ${recs.length}개`;
    videosBox.replaceChildren();
    if (!recs.length) {
      videosBox.append(el('div', { className: 'vbm-empty', textContent: '아직 없어요. 다시보기 화면에서 B를 누르거나 재생바의 책갈피 버튼으로 추가하세요.' }));
      return;
    }
    for (const [key, rec] of recs) {
      const open = expanded.has(key);
      const title = el('a', { className: 'vbm-title', textContent: rec.title || rec.url, title: '처음부터 열기' });
      title.addEventListener('click', () => openVideo(rec, 0));
      const toggleBtn = el('button', { type: 'button', className: 'vbm-toggle', title: '책갈피 보기', textContent: open ? '▾' : '▸' });
      toggleBtn.addEventListener('click', () => {
        if (open) expanded.delete(key); else expanded.add(key);
        renderList();
      });
      const box = el('div', { className: 'vbm-video' }, [
        el('div', { className: 'vbm-vhead' }, [
          el('span', { className: `vbm-badge ${rec.site}`, textContent: SITES[rec.site]?.label || rec.site }),
          el('div', { className: 'vbm-vinfo' }, [
            title,
            el('div', { className: 'vbm-sub', textContent: [rec.channel, `책갈피 ${rec.marks.length}개`].filter(Boolean).join(' · ') }),
          ]),
          toggleBtn,
        ]),
      ]);
      if (open) box.append(...rec.marks.map(m => markRow(key, m, () => openVideo(rec, m.sec))));
      videosBox.append(box);
    }
  };

  const setListOpen = (open) => {
    list.classList.toggle('open', open);
    fab.classList.toggle('on', open);
    renderList();
  };

  fab.addEventListener('click', (e) => { e.stopPropagation(); setListOpen(!list.classList.contains('open')); });
  list.addEventListener('click', e => e.stopPropagation());
  document.addEventListener('click', () => { if (list.classList.contains('open')) setListOpen(false); });
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && list.classList.contains('open')) setListOpen(false); });

  // ---------- 갱신 ----------

  function renderAll() {
    if (panel.classList.contains('open')) renderPanel();
    renderList();
  }

  // 다른 탭에서 바꾼 것도 바로 반영한다
  GM_addValueChangeListener(STORE_KEY, (name, oldValue, newValue, remote) => { if (remote) renderAll(); });

  // 치지직은 다른 영상으로 갈 때, 숲은 다음 영상 자동 재생 때 페이지를 새로 열지 않고 주소만 바꾼다. 주소가 바뀌었는지 주기적으로 본다.
  const onRoute = () => {
    if (!current()) setPanelOpen(false);
    else renderAll();
    handleBm();
  };
  let lastHref = location.href;
  setInterval(() => {
    if (location.href === lastHref) return;
    lastHref = location.href;
    onRoute();
  }, 500);
  onRoute();
})();
