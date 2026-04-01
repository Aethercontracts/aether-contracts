"""
scripts/launcher.py
===================
Expansive interactive Python launcher for the AetherContracts Docker environment.
Features a green/black "matrix" aesthetic, cool typing effects, and 
real-time health polling via subprocess and requests.
"""

import os
import sys
import time
import subprocess
import json
import urllib.request
import urllib.error

# ANSI Escape Codes for the "Coolness"
GREEN = "\033[38;2;0;255;0m"
DARK_GREEN = "\033[38;2;0;150;0m"
CYAN = "\033[38;2;0;255;255m"
WHITE = "\033[38;2;255;255;255m"
RED = "\033[38;2;255;0;0m"
RESET = "\033[0m"

ASCII_ART = f"""{GREEN}
    ___        __  __                 ______            __                  __     
   /   |  ____/ /_/ /_  ___  _____   / ____/___  ____  / /__________ ______/ /_____
  / /| | / __  / __/ / / / |/_/ _ \\ / /   / __ \\/ __ \\/ __/ ___/ __ `/ ___/ __/ ___/
 / ___ |/ /_/ / /_/ /_/ />  </  __// /___/ /_/ / / / / /_/ /  / /_/ / /__/ /_(__  ) 
/_/  |_|\\__,_/\\__/\\__,_/_/|_|\\___/ \\____/\\____/_/ /_/\\__/_/   \\__,_/\\___/\\__/____/ 
                                                                                   
{DARK_GREEN}>> ORBITAL SECURE LINK ESTABLISHED
>> MATHEMATICAL DETERMINISM: ENABLED
>> LLM EMBEDDING: QWEN 2.5 0.5B INSTRUCT
{RESET}"""

def clear_screen():
    os.system('cls' if os.name == 'nt' else 'clear')

def type_text(text, speed=0.015, color=GREEN):
    """Prints text with a retro typing effect."""
    for char in text:
        sys.stdout.write(f"{color}{char}{RESET}")
        sys.stdout.flush()
        time.sleep(speed)
    print()

def check_docker():
    """Verify Docker daemon is active."""
    try:
        subprocess.run(["docker", "info"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False

def check_health(url, name, retries=60, delay=2):
    """Poller that actually checks URL health rather than dumb timeouts."""
    sys.stdout.write(f"{GREEN}[*] Polling {name} at {url} ")
    sys.stdout.flush()
    for i in range(retries):
        try:
            req = urllib.request.Request(url, headers={'User-Agent': 'Aether-Launcher'})
            with urllib.request.urlopen(req, timeout=2) as response:
                if response.getcode() == 200:
                    print(f" {WHITE}[ONLINE]{RESET}")
                    return True
        except Exception:
            pass
        sys.stdout.write(".")
        sys.stdout.flush()
        time.sleep(delay)
    print(f" {RED}[FAILED TO CONNECT]{RESET}")
    return False

def show_menu():
    print(f"{CYAN}=============================================================={RESET}")
    print(f"  {WHITE}[1]{GREEN} INITIALIZE PLATFORM {DARK_GREEN}(Build + Detached Run){RESET}")
    print(f"  {WHITE}[2]{GREEN} EXECUTE DETERMINISM PROOF {DARK_GREEN}(Run Simulator){RESET}")
    print(f"  {WHITE}[3]{GREEN} ACCESS NEURAL INTERFACES {DARK_GREEN}(Dashboard + Status API){RESET}")
    print(f"  {WHITE}[4]{GREEN} VIEW DOCKER LOGS {DARK_GREEN}(Aether Runtime){RESET}")
    print(f"  {WHITE}[5]{GREEN} TERMINATE PLATFORM {DARK_GREEN}(Teardown){RESET}")
    print(f"  {WHITE}[0]{GREEN} SEVER CONNECTION {DARK_GREEN}(Exit){RESET}")
    print(f"{CYAN}=============================================================={RESET}")

def run_command(cmd_list, stream=True):
    """Runs a subprocess command and streams output realistically."""
    if stream:
        process = subprocess.Popen(cmd_list, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, encoding='utf-8', errors='replace')
        for line in process.stdout:
            sys.stdout.write(f"{DARK_GREEN}{line}{RESET}")
            sys.stdout.flush()
        process.wait()
        return process.returncode == 0
    else:
        result = subprocess.run(cmd_list, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        return result.returncode == 0

def main():
    # Force ANSI colors in Windows CMD
    os.system("color 0A")
    clear_screen()
    print(ASCII_ART)
    time.sleep(0.5)

    if not check_docker():
        type_text("[!] CRITICAL ERROR: Docker daemon is unresponsive or not installed.", color=RED)
        type_text("[!] Please launch Docker Desktop and try again.", color=RED)
        input(f"\n{WHITE}Press ENTER to exit...{RESET}")
        return

    while True:
        show_menu()
        choice = input(f"\n{WHITE}AETHER_TERM > {GREEN} ").strip()

        if choice == '1':
            type_text("\n[+] INITIATING SYSTEM BOOT SEQUENCE", speed=0.03, color=CYAN)
            type_text("[*] Compiling Rust binaries and baking Qwen Q5_K_M weights into image...", color=DARK_GREEN)
            success = run_command(["docker", "compose", "up", "--build", "-d"])
            if success:
                type_text("\n[+] CONTAINERS DEPLOYED TO BACKGROUND", color=CYAN)
                type_text("[*] Allowing engines to stabilize and allocate VRAM (ETA ~15s)...\n", color=DARK_GREEN)
                
                # Intelligent Health Polling
                rust_up = check_health("http://localhost:8080/health", "Rust Betti-Topology Scoring Engine")
                llm_up = check_health("http://localhost:8088/health", "Llama.cpp Base LLM Engine")
                api_up = check_health("http://localhost:8090/health", "FastAPI Determinism Bridge")
                
                if rust_up and llm_up and api_up:
                    type_text("\n[+] ALL SUBSYSTEMS NOMINAL. PLATFORM IS FULLY OPERATIONAL.", color=CYAN)
                else:
                    type_text("\n[!] WARNING: Some subsystems failed to report healthy status.", color=RED)
            else:
                type_text("\n[!] CRITICAL FAILURE DURING BUILD/DEPLOYMENT.", color=RED)

        elif choice == '2':
            type_text("\n[+] ENGAGING DETERMINISM OVERRIDE", speed=0.03, color=CYAN)
            type_text("[*] Injecting synthetic payload into simulation harness...", color=DARK_GREEN)
            
            # Use subprocess to run the simulator profile interactively in the terminal
            result = subprocess.run(["docker", "compose", "run", "--rm", "simulator"])
            if result.returncode == 0:
                type_text("\n[+] MATHEMATICAL REPRODUCIBILITY CONFIRMED.", color=CYAN)
            else:
                type_text("\n[!] DETERMINISM CHECK FAILED OR INTERRUPTED.", color=RED)

        elif choice == '3':
            type_text("\n[+] OPENING LOCALHOST ROUTINES", color=CYAN)
            try:
                # Cross-platform open
                if sys.platform == "win32":
                    os.startfile("http://localhost:3000")
                    os.startfile("http://localhost:8090/status")
                elif sys.platform == "darwin":
                    subprocess.Popen(["open", "http://localhost:3000"])
                    subprocess.Popen(["open", "http://localhost:8090/status"])
                else:
                    subprocess.Popen(["xdg-open", "http://localhost:3000"])
                    subprocess.Popen(["xdg-open", "http://localhost:8090/status"])
                type_text("[*] Web interfaces spawned seamlessly.", color=DARK_GREEN)
            except Exception as e:
                type_text(f"[!] Failed to open browser: {e}", color=RED)

        elif choice == '4':
            type_text("\n[+] TAPPING INTO RUNTIME STREAMS (Ctrl+C to exit log tail)", color=CYAN)
            try:
                subprocess.run(["docker", "logs", "-f", "aether-runtime"])
            except KeyboardInterrupt:
                print(f"\n{GREEN}[*] Log stream detached.{RESET}")

        elif choice == '5':
            type_text("\n[+] EXECUTING CLEAN TEARDOWN", speed=0.03, color=CYAN)
            run_command(["docker", "compose", "down"])
            type_text("\n[+] ARCHITECTURE DISMANTLED AND RESOURCES FREED.", color=CYAN)

        elif choice == '0':
            type_text("\n[+] SEVERING SECURE LINK...", speed=0.05, color=DARK_GREEN)
            type_text("GOODBYE.", speed=0.1, color=CYAN)
            break
        
        else:
            type_text("\n[!] INVALID COMMAND SYNTAX.", color=RED)

        print()

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print(f"\n\n{RED}[!] LINK MANUALLY SEVERED BY USER.{RESET}")
        sys.exit(0)
