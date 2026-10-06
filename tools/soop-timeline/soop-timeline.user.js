// ==UserScript==
// @name         SOOP 다시보기 댓글 타임라인
// @namespace    https://github.com/sinwa3/pc-toolbox
// @version      0.1.2
// @description  다시보기 댓글·답글의 타임라인을 전부 모아 플레이어 오른쪽 패널에 보여주고, 클릭하면 그 시간으로 이동한다.
// @match        https://vod.sooplive.com/player/*
// @match        https://vod.sooplive.co.kr/player/*
// @grant        none
// @updateURL    https://raw.githubusercontent.com/sinwa3/pc-toolbox/main/tools/soop-timeline/soop-timeline.user.js
// @downloadURL  https://raw.githubusercontent.com/sinwa3/pc-toolbox/main/tools/soop-timeline/soop-timeline.user.js
// ==/UserScript==

(() => {
  'use strict';

  const ROOT = location.hostname.replace(/^vod\./, '');
  const MEMO_API = `https://stbbs.${ROOT}/api/bbs_memo_action.php`;
  const VIEW_API = `https://api.m.${ROOT}/station/video/a/view`;
  const MIN_TIMELINE = 3;       // 한 사람의 타임스탬프가 이 개수 이상이면 "타임라인", 미만이면 "한 줄 댓글"
  const MAX_PAGES = 200;        // 댓글 페이지(20개씩) 상한
  const REPLY_CONCURRENCY = 4;  // 답글 동시 요청 수

  // 1:23:45 또는 12:34. 앞뒤에 숫자나 콜론이 붙은 건 제외
  const TIME_RE = /(?<![\d:])(?:\d{1,2}:)?\d{1,2}:\d{2}(?![\d:])/g;

  // ---------- 데이터 ----------

  const getTitleNo = () => (location.pathname.match(/\/player\/(\d+)/) || [])[1];

  const post = async (url, params) => {
    const res = await fetch(url, { method: 'POST', body: new URLSearchParams(params), credentials: 'include' });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return res.json();
  };

  const loadVodInfo = async (titleNo) => {
    const d = (await post(VIEW_API, { nTitleNo: titleNo })).data;
    if (!d) throw new Error('영상 정보를 받지 못했어요');
    return {
      stationNo: d.station_no,
      bbsNo: d.bbs_no,
      bjId: d.bj_id,
      boardType: d.board_type,
      maxSec: d.total_file_duration ? d.total_file_duration / 1000 : Infinity,
    };
  };

  const loadComments = async (vod, titleNo, onProgress) => {
    const base = { nStationNo: vod.stationNo, nBbsNo: vod.bbsNo, nTitleNo: titleNo, bj_id: vod.bjId, nBoardType: vod.boardType };
    const parents = [];
    const seen = new Set();
    let lastNo = 0;
    for (let page = 1; page <= MAX_PAGES; page++) {
      const d = (await post(MEMO_API, { ...base, nPageNo: page, nOrderNo: 1, szAction: 'get', nVod: 1, nLastNo: lastNo })).CHANNEL?.DATA;
      const list = d?.list_data || [];
      for (const c of list) {
        if (seen.has(c.p_comment_no)) continue;
        seen.add(c.p_comment_no);
        parents.push(c);
      }
      onProgress(`댓글 ${parents.length}개 불러오는 중…`);
      if (!d?.has_more || !list.length) break;
      lastNo = list[list.length - 1].p_comment_no;
    }

    const queue = parents.filter(c => Number(c.c_comment_cnt) > 0);
    const total = queue.length;
    const replies = [];
    let done = 0;
    let failed = 0;
    const worker = async () => {
      while (queue.length) {
        const c = queue.shift();
        try {
          const d = (await post(MEMO_API, { ...base, szAction: 'get_reply', nParentCommentNo: c.p_comment_no })).CHANNEL?.DATA;
          replies.push(...(d?.list_data || []));
        } catch {
          failed++;
        }
        onProgress(`답글 ${++done}/${total} 불러오는 중…`);
      }
    };
    await Promise.all(Array.from({ length: REPLY_CONCURRENCY }, worker));
    return { comments: [...parents, ...replies], failed };
  };

  // ---------- 파싱 ----------

  const htmlToText = (html) => {
    const withBreaks = String(html || '').replace(/<br\s*\/?>/gi, '\n');
    return new DOMParser().parseFromString(withBreaks, 'text/html').body.textContent || '';
  };

  const toSeconds = (time) => {
    const parts = time.split(':').map(Number);
    if (parts.slice(1).some(n => n >= 60)) return null;
    return parts.reduce((acc, n) => acc * 60 + n, 0);
  };

  // 한 줄에서 타임스탬프마다 "시간 + 그 뒤 소제목"을 뽑는다. 소제목이 앞에 오는 형식도 허용
  const parseEntries = (text, maxSec) => {
    const entries = [];
    for (const line of text.split('\n')) {
      const matches = [...line.matchAll(TIME_RE)];
      matches.forEach((m, i) => {
        const sec = toSeconds(m[0]);
        if (sec === null || sec > maxSec) return;
        const end = i + 1 < matches.length ? matches[i + 1].index : line.length;
        let label = line.slice(m.index + m[0].length, end);
        if (!label.trim() && matches.length === 1) label = line.slice(0, m.index);
        label = label.replace(/^[\s\])\-:|~>]+|[\s[(\-:|~<]+$/g, '');
        const sub = /^(└|↳|ㄴ(?=\s))/.test(label);
        if (sub) label = label.replace(/^(└|↳|ㄴ)\s*/, '');
        entries.push({ sec, time: m[0], label, sub });
      });
    }
    return entries;
  };

  const sortEntries = (entries) => {
    const seen = new Set();
    return entries
      .slice()
      .sort((a, b) => a.sec - b.sec)
      .filter(e => {
        const key = `${e.sec}|${e.label}|${e.nick || ''}`;
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });
  };

  const buildGroups = (comments, maxSec) => {
    const byUser = new Map();
    for (const c of comments) {
      const entries = parseEntries(htmlToText(c.comment), maxSec);
      if (!entries.length) continue;
      const g = byUser.get(c.user_id) || { nick: c.user_nick, entries: [] };
      g.entries.push(...entries);
      byUser.set(c.user_id, g);
    }

    const timelines = [];
    const singles = [];
    for (const g of byUser.values()) {
      if (g.entries.length >= MIN_TIMELINE) timelines.push(g);
      else singles.push(...g.entries.map(e => ({ ...e, sub: false, nick: g.nick })));
    }
    timelines.sort((a, b) => b.entries.length - a.entries.length);

    const groups = timelines.map(g => ({ name: g.nick, entries: sortEntries(g.entries) }));
    if (singles.length) groups.push({ name: '한 줄 댓글', entries: sortEntries(singles), showNick: true });
    if (groups.length > 1) {
      const all = [...byUser.values()].flatMap(g => g.entries.map(e => ({ ...e, sub: false, nick: g.nick })));
      groups.push({ name: '전체 합치기', entries: sortEntries(all), showNick: true });
    }
    return groups;
  };

  // ---------- 플레이어 ----------

  // 긴 다시보기는 여러 파일로 나뉘어 있어 video.currentTime으로는 못 옮긴다.
  // 숲이 댓글 시간 링크(a.time_link[data-time])에 걸어 둔 위임 클릭 처리기를 빌려 쓴다.
  const seek = (sec) => {
    const a = document.createElement('a');
    a.className = 'time_link';
    a.dataset.time = String(sec);
    a.style.display = 'none';
    document.body.appendChild(a);
    a.click();
    a.remove();
  };

  const currentSec = () => {
    const el = document.querySelector('.time-current');
    return el ? toSeconds(el.textContent.trim()) ?? 0 : 0;
  };

  // ---------- 화면 ----------

  const style = document.createElement('style');
  style.textContent = `
    .stl-btn {
      width: 32px; height: 32px; margin-right: 4px; padding: 0;
      display: flex; align-items: center; justify-content: center;
      background: none; border: 0; color: #fff; cursor: pointer; opacity: .85;
    }
    .stl-btn:hover, .stl-btn.on { opacity: 1; color: #9fd36a; }
    #stl-panel {
      position: absolute; top: 0; right: 0; bottom: 64px; z-index: 1000;
      width: min(360px, 45%); display: none; flex-direction: column;
      background: rgba(16, 16, 16, .72); backdrop-filter: blur(6px);
      color: #ddd; font: 15px/1.5 sans-serif; border-bottom-left-radius: 6px;
    }
    #stl-panel.open { display: flex; }
    .stl-head { display: flex; gap: 6px; align-items: center; padding: 8px 8px 6px; }
    .stl-head select, .stl-head input {
      min-width: 0; height: 26px; color: #eee; font-size: 13px;
      background: rgba(255,255,255,.08); border: 1px solid rgba(255,255,255,.18); border-radius: 4px;
    }
    .stl-head select { flex: 1; padding: 0 6px; }
    .stl-head select option { background: #222; }
    .stl-head input { width: 90px; padding: 0 6px; outline: none; }
    .stl-head input:focus { border-color: #9fd36a; }
    .stl-icon {
      flex: none; width: 26px; height: 26px; padding: 0; cursor: pointer; color: #ccc;
      background: none; border: 1px solid rgba(255,255,255,.18); border-radius: 4px;
    }
    .stl-icon:hover { color: #fff; border-color: #fff; }
    .stl-status { padding: 0 10px 6px; font-size: 13px; color: #999; }
    .stl-status:empty { display: none; }
    .stl-list { flex: 1; overflow-y: auto; padding: 0 4px 8px; overscroll-behavior: contain; }
    .stl-row {
      display: flex; gap: 8px; padding: 5px 8px; border-left: 3px solid transparent;
      border-radius: 3px; cursor: pointer;
    }
    .stl-row:hover { background: rgba(255,255,255,.08); }
    .stl-row.sub { padding-left: 24px; color: #aaa; }
    .stl-row.active { background: rgba(159,211,106,.14); border-left-color: #9fd36a; color: #fff; }
    .stl-time { flex: none; color: #9fd36a; font-variant-numeric: tabular-nums; font-weight: bold; }
    .stl-label { flex: 1; word-break: break-all; }
    .stl-nick { flex: none; max-width: 30%; color: #888; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  `;
  document.head.appendChild(style);

  const el = (tag, props = {}, children = []) => {
    const node = Object.assign(document.createElement(tag), props);
    node.append(...children);
    return node;
  };

  const select = el('select', { title: '누구의 타임라인을 볼지' });
  const search = el('input', { type: 'search', placeholder: '검색' });
  const reloadBtn = el('button', { type: 'button', className: 'stl-icon', title: '다시 불러오기', textContent: '↻' });
  const closeBtn = el('button', { type: 'button', className: 'stl-icon', title: '닫기 (T)', textContent: '✕' });
  const status = el('div', { className: 'stl-status' });
  const list = el('div', { className: 'stl-list' });
  const panel = el('div', { id: 'stl-panel' }, [
    el('div', { className: 'stl-head' }, [select, search, reloadBtn, closeBtn]),
    status,
    list,
  ]);

  // 패널 안의 조작이 플레이어(클릭 재생/정지, 휠 볼륨, 단축키)로 새지 않게 막는다
  for (const type of ['click', 'dblclick', 'mousedown', 'mouseup', 'pointerdown', 'pointerup', 'wheel', 'keydown', 'keyup', 'keypress']) {
    panel.addEventListener(type, e => e.stopPropagation());
  }

  const state = { titleNo: null, groups: [], loading: false, activeRow: null, lastScrollByUser: 0 };

  const renderList = () => {
    const group = state.groups[select.selectedIndex];
    list.replaceChildren();
    state.activeRow = null;
    if (!group) return;
    const keyword = search.value.trim().toLowerCase();
    for (const e of group.entries) {
      if (keyword && !`${e.time} ${e.label} ${e.nick || ''}`.toLowerCase().includes(keyword)) continue;
      const row = el('div', { className: `stl-row${e.sub ? ' sub' : ''}` }, [
        el('span', { className: 'stl-time', textContent: e.time }),
        el('span', { className: 'stl-label', textContent: e.label }),
      ]);
      if (group.showNick) row.append(el('span', { className: 'stl-nick', textContent: e.nick }));
      row.dataset.sec = e.sec;
      row.addEventListener('click', () => {
        seek(e.sec);
        markActive(row, false);
      });
      list.append(row);
    }
    updateActive(true);
  };

  const renderGroups = () => {
    const prev = select.value;
    select.replaceChildren(...state.groups.map(g => el('option', { value: g.name, textContent: `${g.name} · ${g.entries.length}개` })));
    if (state.groups.some(g => g.name === prev)) select.value = prev;
    renderList();
  };

  const markActive = (row, scroll) => {
    if (row === state.activeRow) return;
    state.activeRow?.classList.remove('active');
    state.activeRow = row;
    if (!row) return;
    row.classList.add('active');
    // 사용자가 방금 목록을 굴렸으면 따라 내려가지 않는다
    if (scroll && Date.now() - state.lastScrollByUser > 3000) row.scrollIntoView({ block: 'center' });
  };

  // 현재 재생 위치가 속한 항목(시작 시간이 지금보다 앞인 마지막 항목)을 강조
  const updateActive = (scroll) => {
    const now = currentSec();
    let target = null;
    for (const row of list.children) {
      if (Number(row.dataset.sec) <= now) target = row;
      else break;
    }
    markActive(target, scroll);
  };

  const load = async () => {
    const titleNo = getTitleNo();
    if (!titleNo || state.loading) return;
    state.loading = true;
    state.titleNo = titleNo;
    state.groups = [];
    renderGroups();
    try {
      status.textContent = '영상 정보 불러오는 중…';
      const vod = await loadVodInfo(titleNo);
      const { comments, failed } = await loadComments(vod, titleNo, msg => { status.textContent = msg; });
      if (getTitleNo() !== titleNo) return; // 불러오는 사이 다른 영상으로 넘어감
      state.groups = buildGroups(comments, vod.maxSec);
      status.textContent = state.groups.length
        ? (failed ? `답글 ${failed}개를 불러오지 못했어요` : '')
        : `타임라인 댓글이 없어요 (댓글 ${comments.length}개 확인)`;
      renderGroups();
    } catch (err) {
      status.textContent = `불러오기 실패: ${err.message}`;
    } finally {
      state.loading = false;
    }
  };

  // 평소엔 플레이어(#player) 안에 붙인다. 전체화면 요소가 플레이어를 품고 있지 않으면 그쪽으로 옮겨야 보인다.
  const attachPanel = () => {
    const player = document.getElementById('player');
    const fs = document.fullscreenElement;
    const host = fs && fs.tagName !== 'VIDEO' && !(player && fs.contains(player)) ? fs : player;
    if (host && panel.parentElement !== host) host.appendChild(panel);
  };
  document.addEventListener('fullscreenchange', attachPanel);

  let ticker = null;

  const setOpen = (open) => {
    if (open) attachPanel();
    panel.classList.toggle('open', open);
    document.querySelector('.stl-btn')?.classList.toggle('on', open);
    clearInterval(ticker);
    if (!open) return;
    if (state.titleNo !== getTitleNo()) load();
    else updateActive(true);
    ticker = setInterval(() => updateActive(true), 500);
  };

  const toggle = () => setOpen(!panel.classList.contains('open'));

  select.addEventListener('change', renderList);
  search.addEventListener('input', renderList);
  reloadBtn.addEventListener('click', load);
  closeBtn.addEventListener('click', () => setOpen(false));
  list.addEventListener('wheel', () => { state.lastScrollByUser = Date.now(); }, { passive: true });

  document.addEventListener('keydown', (e) => {
    if (e.code !== 'KeyT' || e.ctrlKey || e.altKey || e.metaKey || e.repeat) return;
    const t = e.target;
    if (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName)) return;
    e.preventDefault();
    toggle();
  }, true);

  // 재생바 오른쪽 버튼 묶음 맨 앞에 버튼을 붙인다. 플레이어가 다시 그려지면 다시 붙인다.
  const btn = el('button', { type: 'button', className: 'stl-btn', title: '댓글 타임라인 (T)', innerHTML:
    '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">' +
    '<circle cx="5" cy="6" r="1"/><circle cx="5" cy="12" r="1"/><circle cx="5" cy="18" r="1"/>' +
    '<path d="M10 6h10M10 12h10M10 18h10"/></svg>' });
  btn.addEventListener('click', (e) => { e.stopPropagation(); toggle(); });

  let pending = false;
  new MutationObserver(() => {
    if (pending) return;
    pending = true;
    requestAnimationFrame(() => {
      pending = false;
      const ctrl = document.querySelector('.right_ctrl');
      if (ctrl && !ctrl.contains(btn)) ctrl.prepend(btn);
    });
  }).observe(document.body, { childList: true, subtree: true });
})();
