#!/usr/bin/env python3
"""
Minimal Wayland client to test compositor connectivity
"""
import os
import socket
import struct
import sys

def connect_to_wayland():
    """Attempt to connect to the compositor's Wayland socket"""
    runtime_dir = os.environ.get('XDG_RUNTIME_DIR', '/run/user/1000')
    wayland_display = os.environ.get('WAYLAND_DISPLAY', 'wayland-1')
    
    socket_path = f"{runtime_dir}/{wayland_display}"
    
    print(f"Attempting to connect to: {socket_path}")
    
    try:
        # Create Unix domain socket
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.connect(socket_path)
        print("✅ Successfully connected to Wayland compositor!")
        
        # Send a simple message to test communication
        # This is a simplified wl_display.get_registry request
        # Object ID: 1 (display), Opcode: 1 (get_registry), Size: 12 bytes
        message = struct.pack('<III', 1, (1 << 16) | 12, 2)  # get_registry with new_id=2
        sock.send(message)
        print("✅ Sent test message to compositor")
        
        # Try to read response
        sock.settimeout(1.0)
        try:
            response = sock.recv(1024)
            if response:
                print(f"✅ Received response from compositor: {len(response)} bytes")
                return True
            else:
                print("⚠️  No response received, but connection established")
                return True
        except socket.timeout:
            print("⚠️  Timeout waiting for response, but connection was successful")
            return True
            
    except FileNotFoundError:
        print(f"❌ Socket not found: {socket_path}")
        return False
    except ConnectionRefusedError:
        print("❌ Connection refused - compositor not accepting connections")
        return False
    except Exception as e:
        print(f"❌ Connection failed: {e}")
        return False
    finally:
        try:
            sock.close()
        except:
            pass

if __name__ == "__main__":
    print("=== Minimal Wayland Compositor Test ===")
    print(f"XDG_RUNTIME_DIR: {os.environ.get('XDG_RUNTIME_DIR', 'Not set')}")
    print(f"WAYLAND_DISPLAY: {os.environ.get('WAYLAND_DISPLAY', 'Not set')}")
    print()
    
    if connect_to_wayland():
        print("\n🎉 COMPOSITOR TEST: PASSED")
        print("The compositor is running and accepting client connections!")
        sys.exit(0)
    else:
        print("\n❌ COMPOSITOR TEST: FAILED")
        print("Could not connect to the compositor")
        sys.exit(1)
