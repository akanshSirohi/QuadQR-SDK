'use strict';

// Browser-only structure diagram used by the copied QuadQR demo. The physical
// role calculation mirrors Format v6 geometry so the Structure tab remains a
// useful reference while encoding/decoding itself comes from Rust/WASM.
const ALIGNMENT_PATTERN_AXES = [
  null, [], [6,18], [6,22], [6,26], [6,30], [6,34], [6,22,38], [6,24,42], [6,26,46],
  [6,28,50], [6,30,54], [6,32,58], [6,34,62], [6,26,46,66], [6,26,48,70],
  [6,26,50,74], [6,30,54,78], [6,30,56,82], [6,30,58,86], [6,34,62,90],
  [6,28,50,72,94], [6,26,50,74,98], [6,30,54,78,102], [6,28,54,80,106],
  [6,32,58,84,110], [6,30,58,86,114], [6,34,62,90,118], [6,26,50,74,98,122],
  [6,30,54,78,102,126], [6,26,52,78,104,130], [6,30,56,82,108,134],
  [6,34,60,86,112,138], [6,30,58,86,114,142], [6,34,62,90,118,146],
  [6,30,54,78,102,126,150], [6,24,50,76,102,128,154], [6,28,54,80,106,132,158],
  [6,32,58,84,110,136,162], [6,26,54,82,110,138,166], [6,30,58,86,114,142,170]
];

const TYPE = Object.freeze({
  DATA:'data', FINDER_BLACK:'finder-black', FINDER_WHITE:'finder-white', SEPARATOR:'separator',
  TIMING_BLACK:'timing-black', TIMING_WHITE:'timing-white', ALIGN_BLACK:'align-black', ALIGN_WHITE:'align-white',
  CAL_RED:'cal-red', CAL_GREEN:'cal-green', CAL_BLUE:'cal-blue'
});
const RESERVED_CLASS = Object.freeze({DATA:'data-position',STRUCTURAL:'structural-reserved',CALIBRATION:'calibration-reserved'});
const PALETTE = Object.freeze({black:'#10151d',white:'#f8fbff',red:'#ef233c',green:'#16a34a',blue:'#2563eb',bg:'#071827',grid:'#12344a',cyan:'#56d8ff',yellow:'#ffd166',muted:'#a9c9d7'});

function sizeForVersion(version){
  if(!Number.isInteger(version)||version<1||version>40) throw new Error('Version must be an integer from 1 to 40.');
  return 21+4*(version-1);
}
function reservedClassForType(type){
  if(type===TYPE.DATA) return RESERVED_CLASS.DATA;
  if(type===TYPE.CAL_RED||type===TYPE.CAL_GREEN||type===TYPE.CAL_BLUE) return RESERVED_CLASS.CALIBRATION;
  return RESERVED_CLASS.STRUCTURAL;
}
function alignmentDefinitions(version){
  const size=sizeForVersion(version);
  if(version===1){const center=size-4;return[{row:center,col:center,size:5,primary:true,bootstrap:true,separator:true}];}
  const axes=ALIGNMENT_PATTERN_AXES[version]; const last=axes[axes.length-1]; const out=[];
  for(const row of axes) for(const col of axes){
    if((row===6&&col===6)||(row===6&&col===last)||(row===last&&col===6)) continue;
    out.push({row,col,size:5,primary:row===last&&col===last,bootstrap:false,separator:false});
  }
  return out;
}
function buildLayout(version=7){
  const size=sizeForVersion(version);
  const types=Array.from({length:size},()=>Array(size).fill(TYPE.DATA));
  const reserved=Array.from({length:size},()=>Array(size).fill(false));
  const set=(r,c,type)=>{if(r<0||c<0||r>=size||c>=size)return; reserved[r][c]=true; types[r][c]=type;};
  const finder=(top,left)=>{
    for(let r=-1;r<=7;r++) for(let c=-1;c<=7;c++) if(r===-1||c===-1||r===7||c===7) set(top+r,left+c,TYPE.SEPARATOR);
    for(let r=0;r<7;r++) for(let c=0;c<7;c++){const outer=r===0||c===0||r===6||c===6;const center=r>=2&&r<=4&&c>=2&&c<=4;set(top+r,left+c,outer||center?TYPE.FINDER_BLACK:TYPE.FINDER_WHITE);}
  };
  const alignment=def=>{
    if(def.separator) for(let r=-3;r<=3;r++) for(let c=-3;c<=3;c++) if(Math.abs(r)===3||Math.abs(c)===3) set(def.row+r,def.col+c,TYPE.SEPARATOR);
    for(let r=-2;r<=2;r++) for(let c=-2;c<=2;c++){const outer=Math.abs(r)===2||Math.abs(c)===2;const center=r===0&&c===0;set(def.row+r,def.col+c,outer||center?TYPE.ALIGN_BLACK:TYPE.ALIGN_WHITE);}
  };
  const areaFree=(row,col,h,w)=>{if(row<0||col<0||row+h>size||col+w>size)return false;for(let r=row;r<row+h;r++)for(let c=col;c<col+w;c++)if(reserved[r][c])return false;return true;};
  finder(0,0); finder(0,size-7); finder(size-7,0);
  for(let c=8;c<size-8;c++) set(6,c,c%2===0?TYPE.TIMING_BLACK:TYPE.TIMING_WHITE);
  for(let r=8;r<size-8;r++) set(r,6,r%2===0?TYPE.TIMING_BLACK:TYPE.TIMING_WHITE);
  const alignments=alignmentDefinitions(version); alignments.forEach(alignment);
  let calibration={row:size-6,col:size-13};
  if(!areaFree(calibration.row,calibration.col,2,6)){
    outer: for(let r=size-8;r>=8;r--) for(let c=size-8;c>=8;c--) if(areaFree(r,c,2,6)){calibration={row:r,col:c};break outer;}
  }
  [TYPE.CAL_RED,TYPE.CAL_GREEN,TYPE.CAL_BLUE].forEach((t,i)=>{for(let r=0;r<2;r++)for(let c=0;c<2;c++)set(calibration.row+r,calibration.col+i*2+c,t);});
  const dataPositions=[]; let upward=true; let col=size-1;
  while(col>=0){if(col===6)col--;for(let i=0;i<size;i++){const row=upward?size-1-i:i;for(let dx=0;dx<2;dx++){const c=col-dx;if(c>=0&&!reserved[row][c])dataPositions.push([row,c]);}}upward=!upward;col-=2;}
  const counts={structuralReserved:0,calibrationReserved:0,dataPositions:dataPositions.length};
  for(let r=0;r<size;r++)for(let c=0;c<size;c++){const role=reservedClassForType(types[r][c]);if(role===RESERVED_CLASS.STRUCTURAL)counts.structuralReserved++;else if(role===RESERVED_CLASS.CALIBRATION)counts.calibrationReserved++;}
  return{version,size,types,reserved,alignments,calibration,dataPositions,counts};
}
function deterministicDataColor(row,col,index){let x=(((row+1)*0x45d9f3b)^((col+3)*0x119de1f3)^((index+7)*0x27d4eb2d))>>>0;x^=x>>>16;x=Math.imul(x,0x7feb352d)>>>0;x^=x>>>15;return x&3;}
function fillFor(type,r,c,index){if(type===TYPE.FINDER_BLACK||type===TYPE.TIMING_BLACK||type===TYPE.ALIGN_BLACK)return PALETTE.black;if(type===TYPE.FINDER_WHITE||type===TYPE.SEPARATOR||type===TYPE.TIMING_WHITE||type===TYPE.ALIGN_WHITE)return PALETTE.white;if(type===TYPE.CAL_RED)return PALETTE.red;if(type===TYPE.CAL_GREEN)return PALETTE.green;if(type===TYPE.CAL_BLUE)return PALETTE.blue;return[PALETTE.red,PALETTE.green,PALETTE.blue,PALETTE.white][deterministicDataColor(r,c,index)];}
function generateSVG(options={}){
  const version=Number(options.version??7);const layout=buildLayout(version);const {size,types,dataPositions,alignments,counts}=layout;
  const quiet=4,module=Math.max(3,Math.floor(760/(size+quiet*2))),symbol=(size+quiet*2)*module,matrix=size*module;
  const W=1200,H=Math.max(850,symbol+220),ox=Math.floor((W-symbol)/2),oy=125,mx=ox+quiet*module,my=oy+quiet*module;
  const indexes=new Map(dataPositions.map((p,i)=>[`${p[0]},${p[1]}`,i]));let cells='';
  for(let r=0;r<size;r++)for(let c=0;c<size;c++){const type=types[r][c];const color=fillFor(type,r,c,indexes.get(`${r},${c}`)??-1);cells+=`<rect x="${mx+c*module}" y="${my+r*module}" width="${module}" height="${module}" fill="${color}" stroke="#d7e6ee" stroke-width=".25"/>`;}
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}" role="img" aria-label="QuadQR Format v6 structure Version ${version}"><defs><pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse"><path d="M20 0H0V20" fill="none" stroke="${PALETTE.grid}" stroke-width="1"/></pattern></defs><rect width="${W}" height="${H}" fill="${PALETTE.bg}"/><rect width="${W}" height="${H}" fill="url(#grid)" opacity=".7"/><text x="50" y="58" fill="#eff9ff" font-family="Raleway,Segoe UI,sans-serif" font-size="34" font-weight="750">QuadQR Basic Structure</text><text x="50" y="88" fill="${PALETTE.cyan}" font-family="Raleway,Segoe UI,sans-serif" font-size="15">Format v6 · Version ${version} · ${size}×${size} matrix · ${alignments.length} alignment eyes</text><rect x="${ox}" y="${oy}" width="${symbol}" height="${symbol}" fill="#fff" stroke="${PALETTE.cyan}" stroke-width="2"/>${cells}<rect x="${mx}" y="${my}" width="${matrix}" height="${matrix}" fill="none" stroke="${PALETTE.cyan}" stroke-width="2"/><g font-family="Raleway,Segoe UI,sans-serif" font-size="14" fill="${PALETTE.muted}"><text x="50" y="${H-78}">Structural reserved: ${counts.structuralReserved} cells</text><text x="50" y="${H-54}">Calibration reserved: ${counts.calibrationReserved} cells</text><text x="50" y="${H-30}">Data positions: ${counts.dataPositions} cells · RGBW data colors are illustrative</text></g></svg>`;
}
function mountQuadQRStructure(target,options={}){const el=typeof target==='string'?document.querySelector(target):target;if(!el)throw new Error('Target element not found.');el.innerHTML=generateSVG(options);return el.querySelector('svg');}
window.QuadQRStructureDiagram={generateSVG,buildLayout,alignmentDefinitions,sizeForVersion,reservedClassForType,RESERVED_CLASS,mountQuadQRStructure};
