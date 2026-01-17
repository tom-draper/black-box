use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::event::Event;
use crate::reader::LogReader;

#[derive(Deserialize)]
pub struct EventQueryParams {
    filter: Option<String>,
    #[serde(rename = "type")]
    event_type: Option<String>,
}

pub async fn index() -> HttpResponse {
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Black Box</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
*{margin:0;padding:0;box-sizing:border-box}
body{background:#0d0d0d;color:#e0e0e0;font:12px/1.2 'Fira Code','Consolas','Courier New',monospace;min-width:900px;overflow-x:auto}
.container{padding:4px}
.box{color:#555;white-space:pre;font-family:inherit}
.g{color:#5f5}.dg{color:#0a0}.y{color:#ff0}.r{color:#f55}.c{color:#5cf}.o{color:#fa0}.m{color:#f5f}.w{color:#fff}.d{color:#888}
input,select{background:#1a1a1a;border:1px solid #444;color:#e0e0e0;font:inherit;padding:1px 4px;margin:0 2px}
select{cursor:pointer}
.bordered-section{border-left:1px solid #555;border-right:1px solid #555;margin:0;padding:4px 8px}
.bordered-section canvas{width:100%;height:80px;display:block}
.events-section{max-height:160px;overflow-y:auto;overflow-x:hidden}
.events-hdr{display:flex;align-items:center;gap:4px;width:100%;white-space:nowrap}
.events-hdr .filler{flex:1;overflow:hidden;text-overflow:clip}
.ev{white-space:nowrap;overflow:hidden;text-overflow:ellipsis;padding:1px 0;line-height:1.4}
.lbl{color:#888}
.filter-row{display:flex;align-items:center;gap:8px;flex-wrap:wrap}
.filter-row span{white-space:nowrap}
    </style>
</head>
<body>
<div class="container">
<pre class="box" id="header"></pre>
<pre class="box" id="graphHeader"></pre>
<div class="bordered-section"><canvas id="graph"></canvas></div>
<pre class="box" id="graphFooter"></pre>
<pre class="box" id="eventsHeader"></pre>
<div class="bordered-section events-section" id="logContainer"></div>
<pre class="box" id="footer"></pre>
</div>
<script>
const B='│',H='─',TL='┌',TR='┐',BL='└',BR='┘',LT='├',RT='┤',TB='┬',BT='┴',X='┼';
let ws=null,reconnectTimeout=null,eventBuffer=[],systemMetricsCounter=0,startTime=Date.now();
const MAX_BUFFER=500,MAX_DOM=200;
const metricsHistory={cpu:[],mem:[],disk:[],net:[],maxPoints:60};
let topCpu=[],topMem=[],lastStats=null,canvas=null,ctx=null,totalProcs=0,runningProcs=0;

function bar(pct,w=6){
    const f=Math.round((pct/100)*w);
    return '['+'\u2588'.repeat(f)+'\u2591'.repeat(w-f)+']';
}

function barColor(pct,w=6){
    const f=Math.round((pct/100)*w);
    const c=pct>=90?'r':pct>=70?'y':'g';
    return '[<span class="'+c+'">'+'█'.repeat(f)+'</span><span class="d">'+'░'.repeat(w-f)+'</span>]';
}

function fmt(b){
    if(b===0)return'0B';
    const k=1024,s=['B','K','M','G','T'];
    const i=Math.floor(Math.log(b)/Math.log(k));
    return(b/Math.pow(k,i)).toFixed(i>1?1:0)+s[i];
}

function fmtRate(b){
    if(b===0)return'0B/s';
    const k=1024,s=['B','K','M','G'];
    const i=Math.floor(Math.log(b)/Math.log(k));
    return(b/Math.pow(k,i)).toFixed(i>1?1:0)+s[i]+'/s';
}

function uptime(){
    const s=Math.floor((Date.now()-startTime)/1000);
    const d=Math.floor(s/86400),h=Math.floor((s%86400)/3600),m=Math.floor((s%3600)/60);
    if(d>0)return d+'d '+h+'h '+String(m).padStart(2,'0')+'m';
    if(h>0)return h+'h '+String(m).padStart(2,'0')+'m';
    return m+'m '+String(s%60).padStart(2,'0')+'s';
}

function pad(s,n,r=false){s=String(s);return r?s.padStart(n):s.padEnd(n);}
function line(ch,n){return ch.repeat(n);}
function span(cls,txt){return '<span class="'+cls+'">'+txt+'</span>';}

let savedFilter='',savedEvType='',userScrolled=false;

function render(){
    const W=Math.max(100,Math.floor((window.innerWidth-20)/7.2));
    const e=lastStats||{};
    const wsOk=ws&&ws.readyState===1;
    const wsStatus=wsOk?span('g','●')+' CONNECTED':span('r','●')+' DISCONNECTED';
    const perCore=e.per_core_cpu||[];

    // Save focus state before re-render
    const activeEl=document.activeElement;
    const hadFocus=activeEl&&activeEl.id==='filterInput';
    const cursorPos=hadFocus?activeEl.selectionStart:0;
    const fi=document.getElementById('filterInput');
    const et=document.getElementById('eventType');
    if(fi)savedFilter=fi.value;
    if(et)savedEvType=et.value;
    const filter=savedFilter;
    const evType=savedEvType;
    const now=new Date();
    const timeStr=now.toLocaleTimeString();
    const tz=Intl.DateTimeFormat().resolvedOptions().timeZone||'Unknown';

    // Header
    let o=TL+H+' '+span('g','BLACK-BOX')+' '+span('d','Server Monitor')+' '+line(H,W-30)+TR+'\n';
    o+=B+' '+span('lbl','Status:')+' '+span('g','▲ ACTIVE')+' '+B+' '+span('lbl','WS:')+' '+wsStatus+' '+B+' '+span('lbl','Uptime:')+' '+span('c',uptime())+' '+B+' '+span('d',timeStr)+' '+span('y',tz)+pad('',W-75-tz.length)+B+'\n';

    // Calculate column widths
    const c1=24,c2=22,c3=W-c1-c2-4;

    // Section headers
    o+=LT+H+' '+span('c','CPU')+' '+line(H,c1-6)+TB+H+' '+span('c','MEMORY')+' '+line(H,c2-9)+TB+H+' '+span('c','DISK')+' '+line(H,c3-7)+RT+'\n';

    // CPU section
    const cpu=e.cpu!==undefined?e.cpu.toFixed(1):'--';
    const cpuBar=e.cpu!==undefined?barColor(e.cpu,8):'[░░░░░░░░]';
    const load=e.load!==undefined?e.load.toFixed(2):'--';
    const load5=e.load5!==undefined?e.load5.toFixed(2):'--';
    const load15=e.load15!==undefined?e.load15.toFixed(2):'--';
    const temp=e.cpu_temp?Math.round(e.cpu_temp)+'°C':'--';
    const tempClass=e.cpu_temp?(e.cpu_temp>=80?'r':e.cpu_temp>=60?'y':'g'):'d';

    // Memory section
    const mem=e.mem!==undefined?e.mem.toFixed(1):'--';
    const memBar=e.mem!==undefined?barColor(e.mem,8):'[░░░░░░░░]';
    const memUsed=e.mem_used?fmt(e.mem_used):'--';
    const memTotal=e.mem_total?fmt(e.mem_total):'--';
    const memFree=e.mem_total&&e.mem_used?fmt(e.mem_total-e.mem_used):'--';
    const swap=e.swap_percent||0;
    const swapBar=barColor(swap,8);
    const swapUsed=e.swap_used?fmt(e.swap_used):'0B';
    const swapTotal=e.swap_total?fmt(e.swap_total):'0B';

    // Disk section
    const disk=e.disk!==undefined?e.disk:'--';
    const diskBar=e.disk!==undefined?barColor(e.disk,10):'[░░░░░░░░░░]';
    const diskUsed=e.disk_used?fmt(e.disk_used):'--';
    const diskTotal=e.disk_total?fmt(e.disk_total):'--';
    const diskFree=e.disk_total&&e.disk_used?fmt(e.disk_total-e.disk_used):'--';

    // Build rows
    const cpuRows=[];
    cpuRows.push(span('lbl','Total ')+span('w',pad(cpu+'%',6,true))+' '+cpuBar);
    for(let i=0;i<Math.min(perCore.length,6);i++){
        const v=perCore[i];
        const coreClass=v>=90?'r':v>=70?'y':'g';
        cpuRows.push(span('d','Core'+i)+' '+span(coreClass,pad(v.toFixed(1)+'%',6,true))+' '+barColor(v,8));
    }
    cpuRows.push(span('lbl','Load ')+span('w',load)+span('d','/'+load5+'/'+load15));
    cpuRows.push(span('lbl','Temp ')+span(tempClass,temp));

    const memRows=[];
    memRows.push(span('lbl','Used ')+span('w',pad(mem+'%',6,true))+' '+memBar);
    memRows.push(span('lbl','     ')+span('c',memUsed)+span('d',' / '+memTotal));
    memRows.push(span('lbl','Free ')+span('g',memFree));
    memRows.push(span('lbl','Swap ')+span('w',pad(swap.toFixed(1)+'%',6,true))+' '+swapBar);
    memRows.push(span('lbl','     ')+span('d',swapUsed+' / '+swapTotal));

    // Build disk rows with filesystems (df-style)
    const filesystems=e.filesystems||[];
    const drives=e.per_disk||[];
    const maxIO=Math.max(1,...drives.map(d=>(d.read||0)+(d.write||0)));

    function ioBar(read,write,maxVal,w=6){
        const total=read+write;
        const pct=maxVal>0?(total/maxVal)*100:0;
        const filled=Math.round((pct/100)*w);
        const readPct=total>0?read/total:0.5;
        const readFill=Math.round(filled*readPct);
        const writeFill=filled-readFill;
        return '['+span('g','▓'.repeat(readFill))+span('o','▓'.repeat(writeFill))+span('d','░'.repeat(w-filled))+']';
    }

    const diskRows=[];
    // Show filesystems with usage bars
    for(const fs of filesystems){
        const pct=fs.total>0?Math.round((fs.used/fs.total)*100):0;
        const mount=fs.mount.length>8?fs.mount.substring(0,7)+'…':fs.mount;
        diskRows.push(span('c',pad(mount,8))+' '+barColor(pct,6)+' '+span('w',pad(pct+'%',4,true)));
        diskRows.push(span('d','  ')+span('c',fmt(fs.used))+span('d','/'+fmt(fs.total)));
    }
    // Show I/O per physical drive
    if(drives.length>0){
        diskRows.push(span('lbl','── I/O ──'));
        for(const d of drives){
            const name=d.device.substring(0,6);
            const tempVal=d.temp!==null&&d.temp!==undefined?d.temp:null;
            const tempClass=tempVal?(tempVal>=50?'r':tempVal>=40?'y':'g'):'d';
            const tempStr=tempVal?' '+span(tempClass,Math.round(tempVal)+'°'):'';
            diskRows.push(span('c',pad(name,6))+' '+ioBar(d.read,d.write,maxIO,6)+tempStr);
            diskRows.push(span('d',' ')+span('g','R:')+pad(fmtRate(d.read),7)+span('o','W:')+fmtRate(d.write));
        }
    }

    const maxRows=Math.max(cpuRows.length,memRows.length,diskRows.length);
    for(let i=0;i<maxRows;i++){
        const col1=cpuRows[i]?cpuRows[i]+pad('',c1-stripHtml(cpuRows[i]).length):pad('',c1);
        const col2=memRows[i]?memRows[i]+pad('',c2-stripHtml(memRows[i]).length):pad('',c2);
        const col3=diskRows[i]?diskRows[i]+pad('',c3-stripHtml(diskRows[i]).length):pad('',c3);
        o+=B+col1+B+col2+B+col3+B+'\n';
    }

    // Network | GPU | Processes row
    const hasGpu=e.gpu_temp!==undefined&&e.gpu_temp!==null;
    const nc=20,gc=hasGpu?16:0,pc=W-nc-gc-4-(hasGpu?1:0);

    if(hasGpu){
        o+=LT+H+' '+span('c','NETWORK')+' '+line(H,nc-10)+TB+H+' '+span('m','GPU')+' '+line(H,gc-6)+TB+H+' '+span('c','PROCESSES')+' '+line(H,pc-12)+RT+'\n';
    }else{
        o+=LT+H+' '+span('c','NETWORK')+' '+line(H,nc-10)+TB+H+' '+span('c','PROCESSES')+' '+line(H,W-nc-15)+RT+'\n';
    }

    const rx=e.net_recv||0;
    const tx=e.net_send||0;
    const tcp=e.tcp!==undefined?e.tcp:'--';
    const tcpWait=e.tcp_wait!==undefined?e.tcp_wait:'--';
    const totalNet=rx+tx;

    const netRows=[];
    netRows.push(span('g','▼ RX ')+span('w',pad(fmtRate(rx),9)));
    netRows.push(span('o','▲ TX ')+span('w',pad(fmtRate(tx),9)));
    netRows.push(span('lbl','Tot  ')+span('c',fmtRate(totalNet)));
    netRows.push(span('lbl','TCP  ')+span('w',tcp));
    netRows.push(span('lbl','Wait ')+span('d',tcpWait));

    const gpuRows=[];
    if(hasGpu){
        const gpuTemp=Math.round(e.gpu_temp);
        const gpuTempClass=gpuTemp>=80?'r':gpuTemp>=60?'y':'g';
        gpuRows.push(span('lbl','Temp ')+span(gpuTempClass,gpuTemp+'°C'));
        if(e.mobo_temp){
            const moboTemp=Math.round(e.mobo_temp);
            gpuRows.push(span('lbl','Mobo ')+span('d',moboTemp+'°C'));
        }
        if(e.fans&&e.fans.length>0){
            for(let i=0;i<Math.min(e.fans.length,2);i++){
                const f=e.fans[i];
                const lbl=f.label?f.label.substring(0,4):'Fan'+i;
                gpuRows.push(span('d',lbl+' ')+span('c',f.rpm+'rpm'));
            }
        }
    }

    const procRows=[];
    procRows.push(span('w',totalProcs)+span('d',' procs ')+span('g',runningProcs)+span('d',' run'));
    procRows.push(span('lbl','─ Top CPU ─'));
    topCpu.slice(0,3).forEach(p=>{
        const pctClass=p.cpu>=50?'r':p.cpu>=20?'y':'g';
        procRows.push(span('c',pad(p.name.substring(0,12),12))+' '+span(pctClass,pad(p.cpu.toFixed(1)+'%',6,true)));
    });
    procRows.push(span('lbl','─ Top MEM ─'));
    topMem.slice(0,3).forEach(p=>{
        procRows.push(span('c',pad(p.name.substring(0,12),12))+' '+span('m',pad(fmt(p.mem),6,true)));
    });

    const maxNet=Math.max(netRows.length,gpuRows.length,procRows.length);
    for(let i=0;i<maxNet;i++){
        const col1=netRows[i]?netRows[i]+pad('',nc-stripHtml(netRows[i]).length):pad('',nc);
        const col3=procRows[i]?procRows[i]+pad('',pc-stripHtml(procRows[i]).length):pad('',pc);
        if(hasGpu){
            const col2=gpuRows[i]?gpuRows[i]+pad('',gc-stripHtml(gpuRows[i]).length):pad('',gc);
            o+=B+col1+B+col2+B+col3+B+'\n';
        }else{
            o+=B+col1+B+col3+B+'\n';
        }
    }

    // Users section
    const users=e.users||[];
    if(users.length>0){
        o+=LT+H+' '+span('c','USERS')+' ('+users.length+') '+line(H,W-14-String(users.length).length)+RT+'\n';
        const userStr=users.map(u=>{
            const host=u.remote_host?span('y',u.remote_host):span('d','local');
            return span('w',u.username)+span('d','@')+span('c',u.terminal)+span('d',' [')+host+span('d',']');
        }).join('  ');
        o+=B+' '+userStr+pad('',Math.max(0,W-2-stripHtml(userStr).length))+B+'\n';
    }

    document.getElementById('header').innerHTML=o;

    // Graph section header with legend
    const graphHdr=LT+H+' '+span('c','METRICS')+span('d',' (60s)')+' '+span('g','━')+span('lbl',' CPU')+' '+span('c','━')+span('lbl',' MEM')+' '+span('o','━')+span('lbl',' DISK')+' '+span('m','━')+span('lbl',' NET')+' '+line(H,W-50)+RT;
    document.getElementById('graphHeader').innerHTML=graphHdr;

    // Events header - use flex layout for proper width handling
    let evHdr='<div class="events-hdr">';
    evHdr+=span('c',LT+H)+' '+span('c','EVENTS')+' '+line(H,3)+' ';
    evHdr+=span('lbl','Filter:')+' <input type="text" id="filterInput" value="'+escHtml(filter)+'" style="width:80px" placeholder="search..."> ';
    evHdr+=span('lbl','Type:')+' <select id="eventType"><option value=""'+(evType===''?' selected':'')+'>All</option><option value="process"'+(evType==='process'?' selected':'')+'>Proc</option><option value="security"'+(evType==='security'?' selected':'')+'>Sec</option><option value="anomaly"'+(evType==='anomaly'?' selected':'')+'>Anom</option></select> ';
    evHdr+='<span style="cursor:pointer" class="d" onclick="clearFilter()">[Clr]</span> '+span('d','('+eventBuffer.length+')');
    evHdr+='<span class="filler">'+line(H,200)+'</span>'+span('c',RT)+'</div>';
    document.getElementById('eventsHeader').innerHTML=evHdr;

    // Graph section footer (connects to events header)
    document.getElementById('graphFooter').innerHTML=LT+line(H,W)+RT;

    // Footer
    document.getElementById('footer').innerHTML=BL+line(H,W)+BR;

    // Setup canvas
    setupCanvas();

    // Rebind events and restore focus
    const newFi=document.getElementById('filterInput');
    const newEt=document.getElementById('eventType');
    if(newFi){
        newFi.removeEventListener('input',reloadEvents);
        newFi.addEventListener('input',reloadEvents);
        if(hadFocus){
            newFi.focus();
            newFi.setSelectionRange(cursorPos,cursorPos);
        }
    }
    if(newEt){
        newEt.removeEventListener('change',reloadEvents);
        newEt.addEventListener('change',reloadEvents);
    }
}

function stripHtml(s){return s.replace(/<[^>]*>/g,'');}
function escHtml(s){return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');}

function setupCanvas(){
    canvas=document.getElementById('graph');
    if(canvas){
        const rect=canvas.getBoundingClientRect();
        canvas.width=rect.width*window.devicePixelRatio;
        canvas.height=rect.height*window.devicePixelRatio;
        ctx=canvas.getContext('2d');
        ctx.scale(window.devicePixelRatio,window.devicePixelRatio);
        drawGraph();
    }
}

function drawGraph(){
    if(!canvas||!ctx)return;
    const rect=canvas.getBoundingClientRect();
    const w=rect.width,h=rect.height;
    ctx.fillStyle='#0d0d0d';
    ctx.fillRect(0,0,w,h);

    ctx.strokeStyle='#1a1a1a';
    ctx.lineWidth=1;
    for(let i=0;i<=4;i++){
        const y=(h/4)*i;
        ctx.beginPath();ctx.moveTo(0,y);ctx.lineTo(w,y);ctx.stroke();
    }
    ctx.fillStyle='#333';
    ctx.font='9px monospace';
    ctx.textAlign='right';
    for(let i=0;i<=4;i++){
        ctx.fillText((100-i*25)+'%',w-4,(h/4)*i+10);
    }

    if(metricsHistory.cpu.length<2)return;
    const ps=w/(metricsHistory.maxPoints-1);

    function drawLine(data,color,maxVal=100){
        ctx.strokeStyle=color;ctx.lineWidth=1.5;ctx.beginPath();
        const si=Math.max(0,data.length-metricsHistory.maxPoints);
        for(let i=0;i<data.length-si;i++){
            const x=i*ps,y=h-(data[si+i]/maxVal)*h;
            i===0?ctx.moveTo(x,y):ctx.lineTo(x,y);
        }
        ctx.stroke();
    }
    drawLine(metricsHistory.cpu,'#55ff55');
    drawLine(metricsHistory.mem,'#55ccff');
    drawLine(metricsHistory.disk,'#ffaa00');
    if(metricsHistory.net.length>0){
        const maxNet=Math.max(...metricsHistory.net,1);
        drawLine(metricsHistory.net,'#ff55ff',maxNet);
    }
}

function connectWebSocket(){
    const protocol=window.location.protocol==='https:'?'wss:':'ws:';
    ws=new WebSocket(protocol+'//'+window.location.host+'/ws');
    ws.onopen=()=>{render();};
    ws.onmessage=(ev)=>{try{addEvent(JSON.parse(ev.data));}catch(e){}};
    ws.onerror=()=>{render();};
    ws.onclose=()=>{render();setTimeout(connectWebSocket,5000);};
}

function updateStats(e){
    if(e.type!=='SystemMetrics')return;
    lastStats=e;
    metricsHistory.cpu.push(e.cpu);
    metricsHistory.mem.push(e.mem);
    metricsHistory.disk.push(e.disk);
    metricsHistory.net.push((e.net_recv||0)+(e.net_send||0));
    if(metricsHistory.cpu.length>metricsHistory.maxPoints){
        metricsHistory.cpu.shift();metricsHistory.mem.shift();metricsHistory.disk.shift();metricsHistory.net.shift();
    }
    render();
}

function updateProcesses(e){
    if(e.type!=='ProcessSnapshot')return;
    const procs=e.processes||[];
    totalProcs=procs.length;
    runningProcs=procs.filter(p=>p.state==='R').length;
    topCpu=procs.slice().sort((a,b)=>b.cpu_percent-a.cpu_percent).slice(0,3).map(p=>({name:p.name,cpu:p.cpu_percent,mem:p.mem_bytes}));
    topMem=procs.slice().sort((a,b)=>b.mem_bytes-a.mem_bytes).slice(0,3).map(p=>({name:p.name,cpu:p.cpu_percent,mem:p.mem_bytes}));
}

function addEvent(event){
    updateStats(event);
    updateProcesses(event);

    if(event.type==='SystemMetrics'){
        systemMetricsCounter++;
        if(systemMetricsCounter%10!==0)return;
    }
    if(event.type==='ProcessSnapshot')return;

    const filter=(document.getElementById('filterInput')||{}).value||'';
    const evType=(document.getElementById('eventType')||{}).value||'';
    if(!matchesFilter(event,filter.toLowerCase(),evType))return;

    eventBuffer.push(event);
    if(eventBuffer.length>MAX_BUFFER)eventBuffer.shift();

    const container=document.getElementById('logContainer');
    if(!container)return;
    const entry=createLogEntry(event);
    if(entry)container.appendChild(entry);
    // Only auto-scroll if user is near bottom
    const nearBottom=container.scrollHeight-container.scrollTop-container.clientHeight<50;
    if(nearBottom)container.scrollTop=container.scrollHeight;
    while(container.children.length>MAX_DOM)container.removeChild(container.firstChild);
}

function matchesFilter(e,filter,evType){
    if(evType){
        const map={system:'SystemMetrics',process:'ProcessLifecycle',security:'SecurityEvent',anomaly:'Anomaly'};
        if(e.type!==map[evType])return false;
    }
    if(filter&&!JSON.stringify(e).toLowerCase().includes(filter))return false;
    return true;
}

function createLogEntry(e){
    const ts=e.timestamp||'',type=e.type||'';
    if(type==='ProcessSnapshot'||type==='unknown')return null;
    const div=document.createElement('div');
    div.className='ev';
    const time='<span class="d">'+ts.substring(11,19)+'</span>';
    let txt='';

    if(type==='SystemMetrics'){
        txt=time+' <span class="dg">[SYS]</span> <span class="g">CPU:</span>'+e.cpu.toFixed(1)+'% <span class="c">Mem:</span>'+e.mem.toFixed(1)+'% <span class="lbl">Load:</span>'+e.load.toFixed(2);
    }else if(type==='ProcessLifecycle'){
        const sym=e.kind==='Started'?'<span class="g">+</span>':e.kind==='Exited'?'<span class="r">-</span>':e.kind==='Stuck'?'<span class="y">D</span>':'<span class="r">Z</span>';
        txt=time+' <span class="o">[PROC]</span> '+sym+' <span class="c">'+e.name+'</span> <span class="d">(pid '+e.pid+')</span>';
    }else if(type==='SecurityEvent'){
        const isOk=e.kind==='SshLoginSuccess';
        const lbl=e.kind==='SshLoginSuccess'?'SSH OK':e.kind==='SshLoginFailure'?'SSH FAIL':e.kind==='SudoCommand'?'SUDO':e.kind;
        txt=time+' <span class="m">[SEC]</span> <span class="'+(isOk?'g':'r')+'">'+lbl+'</span> <span class="c">'+e.user+'</span>'+(e.source_ip?' <span class="d">from</span> <span class="y">'+e.source_ip+'</span>':'');
    }else if(type==='Anomaly'){
        const isCrit=e.severity==='Critical';
        txt=time+' <span class="'+(isCrit?'r':'y')+'">['+( isCrit?'CRIT':'WARN')+']</span> '+e.message;
    }else{return null;}

    div.innerHTML=txt;
    return div;
}

function clearFilter(){
    const fi=document.getElementById('filterInput');
    const et=document.getElementById('eventType');
    if(fi){fi.value='';savedFilter='';}
    if(et){et.value='';savedEvType='';}
    reloadEvents();
}

function reloadEvents(){
    const container=document.getElementById('logContainer');
    if(!container)return;
    const fi=document.getElementById('filterInput');
    const et=document.getElementById('eventType');
    if(fi)savedFilter=fi.value;
    if(et)savedEvType=et.value;
    container.innerHTML='';
    const filter=savedFilter.toLowerCase();
    const evType=savedEvType;
    const start=Math.max(0,eventBuffer.length-MAX_DOM);
    for(let i=start;i<eventBuffer.length;i++){
        if(matchesFilter(eventBuffer[i],filter,evType)){
            const entry=createLogEntry(eventBuffer[i]);
            if(entry)container.appendChild(entry);
        }
    }
    container.scrollTop=container.scrollHeight;
}

window.addEventListener('resize',()=>{render();setupCanvas();});
render();
connectWebSocket();
setTimeout(setupCanvas,100);
</script>
</body>
</html>"#;

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

pub async fn api_events(
    reader: web::Data<LogReader>,
    query: web::Query<EventQueryParams>,
) -> HttpResponse {
    let filter = query.filter.as_ref().map(|s| s.to_lowercase());
    let event_type = query.event_type.as_deref();

    let events = match reader.read_all_events() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error reading events: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": format!("Failed to read events: {}", e)}));
        }
    };

    // Convert to JSON-serializable format
    let mut json_events = Vec::new();

    for event in events.iter().rev().take(1000) {
        if let Some(json_event) = event_to_json(event, &filter, event_type) {
            json_events.push(json_event);
        }
    }

    json_events.reverse();

    HttpResponse::Ok().json(json_events)
}

fn event_to_json(
    event: &Event,
    filter: &Option<String>,
    event_type_filter: Option<&str>,
) -> Option<serde_json::Value> {
    use time::format_description::well_known::Rfc3339;

    match event {
        Event::SystemMetrics(m) => {
            if event_type_filter.is_some() && event_type_filter != Some("system") {
                return None;
            }

            let disk_pct = if m.disk_total_bytes > 0 {
                (m.disk_used_bytes as f64 / m.disk_total_bytes as f64) * 100.0
            } else {
                0.0
            };

            let mem_pct = if m.mem_total_bytes > 0 {
                (m.mem_used_bytes as f64 / m.mem_total_bytes as f64) * 100.0
            } else {
                0.0
            };

            Some(serde_json::json!({
                "type": "SystemMetrics",
                "timestamp": m.ts.format(&Rfc3339).ok()?,
                "cpu": m.cpu_usage_percent,
                "per_core_cpu": m.per_core_usage,
                "mem": mem_pct,
                "mem_used": m.mem_used_bytes,
                "mem_total": m.mem_total_bytes,
                "load": m.load_avg_1m,
                "load5": m.load_avg_5m,
                "load15": m.load_avg_15m,
                "disk": disk_pct.round(),
                "disk_used": m.disk_used_bytes,
                "disk_total": m.disk_total_bytes,
                "per_disk": m.per_disk_metrics.iter().map(|d| serde_json::json!({
                    "device": d.device_name,
                    "read": d.read_bytes_per_sec,
                    "write": d.write_bytes_per_sec,
                    "temp": d.temp_celsius,
                })).collect::<Vec<_>>(),
                "filesystems": m.filesystems.iter().map(|fs| serde_json::json!({
                    "filesystem": fs.filesystem,
                    "mount": fs.mount_point,
                    "total": fs.total_bytes,
                    "used": fs.used_bytes,
                    "available": fs.available_bytes,
                })).collect::<Vec<_>>(),
                "tcp": m.tcp_connections,
                "tcp_wait": m.tcp_time_wait,
                "net_recv": m.net_recv_bytes_per_sec,
                "net_send": m.net_send_bytes_per_sec,
                "cpu_temp": m.temps.cpu_temp_celsius,
                "per_core_temps": m.temps.per_core_temps,
                "gpu_temp": m.temps.gpu_temp_celsius,
                "mobo_temp": m.temps.motherboard_temp_celsius,
                "fans": m.fans.iter().map(|f| serde_json::json!({
                    "label": f.label,
                    "rpm": f.rpm,
                })).collect::<Vec<_>>(),
                "users": m.logged_in_users.iter().map(|u| serde_json::json!({
                    "username": u.username,
                    "terminal": u.terminal,
                    "remote_host": u.remote_host,
                })).collect::<Vec<_>>(),
            }))
        }
        Event::ProcessLifecycle(p) => {
            if event_type_filter.is_some() && event_type_filter != Some("process") {
                return None;
            }

            let text = format!("{:?} {} {}", p.kind, p.name, p.pid);
            if let Some(f) = filter {
                if !text.to_lowercase().contains(f) {
                    return None;
                }
            }

            Some(serde_json::json!({
                "type": "ProcessLifecycle",
                "timestamp": p.ts.format(&Rfc3339).ok()?,
                "kind": format!("{:?}", p.kind),
                "pid": p.pid,
                "name": p.name,
            }))
        }
        Event::SecurityEvent(s) => {
            if event_type_filter.is_some() && event_type_filter != Some("security") {
                return None;
            }

            let text = format!("{} {} {:?}", s.user, s.message, s.kind);
            if let Some(f) = filter {
                if !text.to_lowercase().contains(f) {
                    return None;
                }
            }

            Some(serde_json::json!({
                "type": "SecurityEvent",
                "timestamp": s.ts.format(&Rfc3339).ok()?,
                "kind": format!("{:?}", s.kind),
                "user": s.user,
                "source_ip": s.source_ip,
                "message": s.message,
            }))
        }
        Event::Anomaly(a) => {
            if event_type_filter.is_some() && event_type_filter != Some("anomaly") {
                return None;
            }

            let text = format!("{:?} {}", a.kind, a.message);
            if let Some(f) = filter {
                if !text.to_lowercase().contains(f) {
                    return None;
                }
            }

            Some(serde_json::json!({
                "type": "Anomaly",
                "timestamp": a.ts.format(&Rfc3339).ok()?,
                "severity": format!("{:?}", a.severity),
                "kind": format!("{:?}", a.kind),
                "message": a.message,
            }))
        }
        Event::ProcessSnapshot(p) => {
            if event_type_filter.is_some() && event_type_filter != Some("process") {
                return None;
            }

            Some(serde_json::json!({
                "type": "ProcessSnapshot",
                "timestamp": p.ts.format(&Rfc3339).ok()?,
                "count": p.processes.len(),
                "processes": p.processes.iter().map(|proc| serde_json::json!({
                    "pid": proc.pid,
                    "name": proc.name,
                    "cmdline": proc.cmdline,
                    "state": proc.state,
                    "cpu_percent": proc.cpu_percent,
                    "mem_bytes": proc.mem_bytes,
                    "num_threads": proc.num_threads,
                })).collect::<Vec<serde_json::Value>>(),
            }))
        }
    }
}
