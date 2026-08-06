import gdb
import socket

sock = socket.create_connection(("localhost", 4444))
sock.setblocking(False)

class Setup(gdb.Command):
    """Default setup for connecting to qemu"""
    def __init__(self):
        super().__init__("setup", command_class=gdb.COMMAND_USER)
    
    def invoke(self, argument, from_tty):
        gdb.execute("set architecture i386")
        gdb.execute("symbol-file build/eebos.bin")
        gdb.execute("target remote localhost:1234")
        gdb.execute("b core64::make_fncall")

class Do(gdb.Command):
    def __init__(self):
        super().__init__("do", command_class=gdb.COMMAND_USER)
    
    def invoke(self, argument, from_tty):
        # there is a race condition but it doesn't matter
        data = b''
        while True:
            try:
                data += sock.recv(1024)
            except BlockingIOError:
                break
        for line in data.splitlines():
            gdb.execute(line.decode(), to_string=True)
        
        gdb.execute("b rmpanic_mark")

Setup()
Do()

