"""Run the native drawing workload and record process CPU time (standard library only)."""
import argparse
import ctypes
import json
import os
from pathlib import Path
import subprocess
import time

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable',type=Path)
    parser.add_argument('report',type=Path)
    args=parser.parse_args(); report=args.report.resolve()
    report.parent.mkdir(parents=True,exist_ok=True)
    if report.exists(): report.unlink()
    started=time.perf_counter()
    process=subprocess.Popen([str(args.executable.resolve()),'--benchmark',str(report)])
    cpu_ms=None
    if os.name=='nt':
        from ctypes import wintypes
        kernel=ctypes.WinDLL('kernel32',use_last_error=True)
        kernel.OpenProcess.argtypes=[wintypes.DWORD,wintypes.BOOL,wintypes.DWORD];kernel.OpenProcess.restype=wintypes.HANDLE
        kernel.GetProcessTimes.argtypes=[wintypes.HANDLE,*([ctypes.POINTER(wintypes.FILETIME)]*4)]
        kernel.GetProcessTimes.restype=wintypes.BOOL
        kernel.CloseHandle.argtypes=[wintypes.HANDLE];kernel.CloseHandle.restype=wintypes.BOOL
        handle=kernel.OpenProcess(0x1000,False,process.pid)
        if not handle: raise ctypes.WinError(ctypes.get_last_error())
        try:
            try:
                code=process.wait(timeout=40)
            except subprocess.TimeoutExpired:
                process.kill(); process.wait(); raise
            creation,exit_time,system,user=[wintypes.FILETIME() for _ in range(4)]
            if not kernel.GetProcessTimes(handle,*map(ctypes.byref,[creation,exit_time,system,user])): raise ctypes.WinError(ctypes.get_last_error())
            ticks=lambda value:(value.dwHighDateTime<<32)|value.dwLowDateTime
            cpu_ms=(ticks(system)+ticks(user))/10000
        finally: kernel.CloseHandle(handle)
    else:
        import resource
        before=resource.getrusage(resource.RUSAGE_CHILDREN)
        try:
            code=process.wait(timeout=40)
        except subprocess.TimeoutExpired:
            process.kill(); process.wait(); raise
        after=resource.getrusage(resource.RUSAGE_CHILDREN)
        cpu_ms=(after.ru_utime+after.ru_stime-before.ru_utime-before.ru_stime)*1000
    if code or not report.is_file(): raise RuntimeError('Drawing workload failed or timed out without a report')
    data=json.loads(report.read_text(encoding='utf-8'))
    if data.get('error') or data.get('samples')!=600 or data.get('frame_updates',0)==0:
        raise RuntimeError('Drawing workload did not complete successfully: '+str(data))
    data.update(process_cpu_ms=cpu_ms,process_wall_ms=(time.perf_counter()-started)*1000,logical_processors=os.cpu_count())
    report.write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8')
    print(json.dumps(data,ensure_ascii=False,indent=2))
if __name__=='__main__': main()
