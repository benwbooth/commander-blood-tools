import ctypes, subprocess, time, os, re, struct, sys
ANCHOR = bytes.fromhex("00000009030c08030b07040b07030a06"); DS_ANCHOR = 0x5D98; DS_SEG = 0x0CE2
def main():
    game=os.path.realpath(sys.argv[1]); wait=int(sys.argv[2]); outp=sys.argv[3]
    libc=ctypes.CDLL("libc.so.6",use_errno=True); libc.ptrace.restype=ctypes.c_long
    libc.ptrace.argtypes=[ctypes.c_long,ctypes.c_long,ctypes.c_void_p,ctypes.c_void_p]
    env=dict(os.environ); env["DISPLAY"]=env.get("DISPLAY",":58"); env["SDL_VIDEODRIVER"]="x11"
    xvfb=subprocess.Popen(["Xvfb",env["DISPLAY"],"-screen","0","800x600x24"],stdout=-3,stderr=-3); time.sleep(3)
    db=subprocess.Popen(["dosbox-x","-set","sdl","output=surface","-c",f"mount c {game}","-c","c:","-c","BLOODPRG.EXE"],stdout=-3,stderr=-3,env=env)
    time.sleep(wait); pid=db.pid
    try:
        if libc.ptrace(16,pid,None,None)!=0: print("attach fail"); return
        os.waitpid(pid,0); mem=open(f"/proc/{pid}/mem","rb"); anchor=None
        for line in open(f"/proc/{pid}/maps"):
            pr=line.split()
            if 'r' not in pr[1] or '-' not in pr[0]: continue
            a,b=[int(x,16) for x in pr[0].split('-')]
            if b-a>300_000_000: continue
            try: mem.seek(a); buf=mem.read(b-a)
            except: continue
            for m in re.finditer(re.escape(ANCHOR),buf):
                A=a+m.start(); mem.seek(A-(DS_ANCHOR-0x2F69)); z=struct.unpack('<h',mem.read(2))[0]
                if z==0: anchor=A; break
            if anchor: break
        if not anchor: print("anchor not found"); return
        ds_base = anchor - DS_ANCHOR                 # linear addr of DS:0x0000
        dos_base = ds_base - DS_SEG*16               # linear addr of DOS physical 0
        # read far ptrs
        def rd(off,n): mem.seek(ds_base+off); return mem.read(n)
        for name,off in [("bb_5229",0x5229),("scr_521d",0x521d),("pg_5221",0x5221)]:
            o,s=struct.unpack('<HH', rd(off,4)); print(f"{name}: {s:04x}:{o:04x} -> lin 0x{dos_base+s*16+o:08x}")
        o,s=struct.unpack('<HH', rd(0x5229,4))
        buf_lin = dos_base + s*16 + o
        mem.seek(buf_lin); fb = mem.read(64000)
        open(outp+".fb","wb").write(fb)
        mem.seek(ds_base+0x5b58); open(outp+".pal","wb").write(mem.read(768))
        nz=sum(1 for x in fb if x)
        print(f"framebuffer 0x5229: {nz}/64000 nonzero, distinct vals={len(set(fb))}")
    finally:
        libc.ptrace(17,pid,None,None); db.kill(); xvfb.kill()
main()
