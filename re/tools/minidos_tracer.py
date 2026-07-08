from unicorn import *
from unicorn.x86_const import *
import struct, os, glob
EXE = open("re/bin/BLOODPRG.EXE", "rb").read()
GAME = "output/_tmp_iso"
(sig,lastpg,pages,nreloc,hdrpar,mn,mx,ss0,sp0,ck,ip0,cs0,reloctab)=struct.unpack("<2sHHHHHHHHHHHH",EXE[:26])
modstart=hdrpar*16
relocs=[struct.unpack("<HH",EXE[reloctab+i*4:reloctab+i*4+4]) for i in range(nreloc)]
MEM=0x300000; PSP=0x0100; LOAD=PSP+0x10
mu=Uc(UC_ARCH_X86,UC_MODE_16); mu.mem_map(0,MEM)
img=bytearray(EXE[modstart:])
for o,s in relocs:
    fo=s*16+o
    if fo+2<=len(img): struct.pack_into("<H",img,fo,(struct.unpack("<H",img[fo:fo+2])[0]+LOAD)&0xFFFF)
mu.mem_write(LOAD*16,bytes(img)); mu.mem_write(PSP*16,b"\xCD\x20")
mu.reg_write(UC_X86_REG_CS,LOAD+cs0); mu.reg_write(UC_X86_REG_IP,ip0)
mu.reg_write(UC_X86_REG_SS,LOAD+ss0); mu.reg_write(UC_X86_REG_SP,sp0)
mu.reg_write(UC_X86_REG_DS,PSP); mu.reg_write(UC_X86_REG_ES,PSP)
# free memory arena for AH=48 allocs
next_seg=[0x3000]; handles={}; nexth=[5]
files={os.path.basename(p).upper():p for p in glob.glob(GAME+"/*")}
def rd_str(addr):
    b=b"";
    while True:
        c=mu.mem_read(addr,1); 
        if c==b"\x00": break
        b+=c; addr+=1
    return b.decode("latin1")
def setCF(v):
    fl=mu.reg_read(UC_X86_REG_EFLAGS); mu.reg_write(UC_X86_REG_EFLAGS,(fl|1) if v else (fl&~1))
opened=[]
def onintr(u,intno,ud):
    ax=u.reg_read(UC_X86_REG_AX); ah=(ax>>8)&0xFF; al=ax&0xFF
    if intno==0x21:
        if ah==0x48: u.reg_write(UC_X86_REG_AX,next_seg[0]); next_seg[0]+=u.reg_read(UC_X86_REG_BX); setCF(0)
        elif ah==0x4A: setCF(0)
        elif ah in (0x25,0x1A,0x2C,0x2A): setCF(0)
        elif ah==0x35: u.reg_write(UC_X86_REG_BX,0); u.reg_write(UC_X86_REG_ES,0); setCF(0)
        elif ah==0x30: u.reg_write(UC_X86_REG_AX,0x0005); setCF(0)
        elif ah==0x3D:  # open
            fn=os.path.basename(rd_str(u.reg_read(UC_X86_REG_DS)*16+u.reg_read(UC_X86_REG_DX))).upper()
            p=files.get(fn)
            if p: h=nexth[0]; nexth[0]+=1; handles[h]=open(p,"rb"); u.reg_write(UC_X86_REG_AX,h); setCF(0); opened.append(fn)
            else: u.reg_write(UC_X86_REG_AX,2); setCF(1)
        elif ah==0x3F:  # read
            h=u.reg_read(UC_X86_REG_BX); n=u.reg_read(UC_X86_REG_CX); buf=u.reg_read(UC_X86_REG_DS)*16+u.reg_read(UC_X86_REG_DX)
            if h in handles:
                data=handles[h].read(n); u.mem_write(buf,data); u.reg_write(UC_X86_REG_AX,len(data)); setCF(0)
            else: u.reg_write(UC_X86_REG_AX,0); setCF(1)
        elif ah==0x42:  # seek
            h=u.reg_read(UC_X86_REG_BX); off=(u.reg_read(UC_X86_REG_CX)<<16)|u.reg_read(UC_X86_REG_DX)
            if h in handles: handles[h].seek(off,al); pos=handles[h].tell(); u.reg_write(UC_X86_REG_AX,pos&0xFFFF); u.reg_write(UC_X86_REG_DX,(pos>>16)&0xFFFF); setCF(0)
            else: setCF(1)
        elif ah==0x3E:  # close
            h=u.reg_read(UC_X86_REG_BX)
            if h in handles: handles[h].close(); del handles[h]
            setCF(0)
        elif ah==0x4E or ah==0x4F:  # findfirst/next - report not found (game falls back)
            u.reg_write(UC_X86_REG_AX,18); setCF(1)
        else: setCF(0)
    elif intno==0x2F: u.reg_write(UC_X86_REG_AX,ax&0xFF00); setCF(0)  # XMS/CD absent (AL=0)
    elif intno==0x67: ah2=ah; u.reg_write(UC_X86_REG_AX,0x80<<8); setCF(0)  # EMS: status 0x80 (not found)
    elif intno==0x33: u.reg_write(UC_X86_REG_AX,0); setCF(0)  # mouse absent
    elif intno==0x16: u.reg_write(UC_X86_REG_AX,0); setCF(1)  # no key
    elif intno==0x1A: setCF(0)
    # int10h: no-op (video tracked via mem)
mu.hook_add(UC_HOOK_INTR,onintr)
n=[0]
def onc(u,a,s,ud): n[0]+=1
mu.hook_add(UC_HOOK_CODE,onc)
try:
    mu.emu_start(mu.reg_read(UC_X86_REG_CS)*16+mu.reg_read(UC_X86_REG_IP),0,count=5000000)
    print(f"ran to completion/halt after {n[0]} insns")
except UcError as e:
    cs=mu.reg_read(UC_X86_REG_CS); ip=mu.reg_read(UC_X86_REG_IP)
    print(f"STOP after {n[0]} insns: {e} @ {cs:#06x}:{ip:#06x}")
print(f"files OPENED by the binary: {opened[:15]}")
