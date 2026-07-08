import ctypes, subprocess, time, os, re, struct, sys
ANCHOR = bytes.fromhex("00000009030c08030b07040b07030a06"); DS_ANCHOR = 0x5D98
def main():
    game=os.path.realpath(sys.argv[1]); wait=int(sys.argv[2]); outp=sys.argv[3]
    libc=ctypes.CDLL("libc.so.6",use_errno=True); libc.ptrace.restype=ctypes.c_long
    libc.ptrace.argtypes=[ctypes.c_long,ctypes.c_long,ctypes.c_void_p,ctypes.c_void_p]
    env=dict(os.environ); env["DISPLAY"]=env.get("DISPLAY",":58"); env["SDL_VIDEODRIVER"]="x11"
    xvfb=subprocess.Popen(["Xvfb",env["DISPLAY"],"-screen","0","800x600x24"],stdout=-3,stderr=-3)
    time.sleep(3)
    db=subprocess.Popen(["dosbox-x","-set","sdl","output=surface","-set","sdl","windowresolution=640x400",
        "-set","render","aspect=false","-c",f"mount c {game}","-c","c:","-c","BLOODPRG.EXE"],stdout=-3,stderr=-3,env=env)
    time.sleep(wait); pid=db.pid
    # paired: screenshot RIGHT before the ptrace read
    subprocess.run(["import","-window","root","-crop","640x400+80+100","+repage","-filter","Box",
        "-resize","320x200!",outp+".png"],env=env,stdout=-3,stderr=-3)
    try:
        if libc.ptrace(16,pid,None,None)!=0: print("attach fail"); return
        os.waitpid(pid,0); mem=open(f"/proc/{pid}/mem","rb"); best=None
        for line in open(f"/proc/{pid}/maps"):
            pr=line.split()
            if 'r' not in pr[1] or '-' not in pr[0]: continue
            a,b=[int(x,16) for x in pr[0].split('-')]
            if b-a>300_000_000: continue
            try: mem.seek(a); buf=mem.read(b-a)
            except: continue
            for m in re.finditer(re.escape(ANCHOR),buf):
                A=a+m.start(); mem.seek(A-(DS_ANCHOR-0x2F69)); z=struct.unpack('<h',mem.read(2))[0]
                if z==0: best=A; break
            if best: break
        if not best: print("anchor not found"); return
        mem.seek(best-(DS_ANCHOR-0x5b58)); pal=mem.read(768)
        open(outp+".pal","wb").write(pal)
        nz=sum(1 for x in pal if x)
        print(f"paired: {outp}.png + palette ({nz}/768 nonzero, max={max(pal)})")
    finally:
        libc.ptrace(17,pid,None,None); db.kill(); xvfb.kill()
main()
