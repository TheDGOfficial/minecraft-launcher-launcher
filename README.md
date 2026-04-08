# minecraft-launcher-launcher

Minecraft launcher launcher - a wrapper that launches the Minecraft launcher with tweaks for performance and stability, especially under Linux. 

# Features

- Launches the launcher with custom environment variables to improve performance and stability.
 - `vblank_mode=0` / `__GL_SYNC_TO_VBLANK=0` : Removes the forced frame limit/VSync, improving performance and stability due to less input lag and more responsive game.
 - `ALSOFT_DRIVERS=pipewire` : Makes OpenAL use the native Pipewire backend instead of going through PulseAudio compatibility layer, improving sound throughput and lowering latency.
 - `MESA_NO_ERROR=true` : Disables error checking to improve performance on a production run.
 - `MESA_GL_VERSION_OVERRIDE=4.3` : Allows launching the game on older GPUs, where Mesa provides partial support or fallback paths for newer OpenGL methods, while not compromising anything if your GPU already supports modern OpenGL.

 and more environment variables are overidden for performance and stability.

- Watches process creation to detect java process opening, inspects to make sure it's the Minecraft's java game process, and then kills the launcher after game is open, to save on system resources. The launcher uses CEF (Chromium Embedded Framework), so while it's not Electron it still uses lots of memory in the background, as much as 300 MB, which you can let your game or OS file-system caches or other apps like Firefox take by closing the useless Minecraft Launcher process after java has fully launched the game.
 Kills all processes related to the launcher, including stealth and CEF processes.

- Removes the JavaCheck.jar file to bypass the java runtime check and use any Java version and vendor, even locally built ones.

- Backups launcher_profiles.json to avoid resetting game configurations or settings automatically.

- Launches the launcher with nice priority of -6 for better responsiveness of both the launcher and the game.

- Launches the launcher with ionice priority of -c2 and -n0 (best-effort highest priority, but not realtime) for faster load times and better I/O latency.

- Automatic install to /usr/bin/minecraft-launcher and copies the real launcher to /usr/bin/minecraft-launcher-real.

# Requirements

The launcher wrapper process requires sudo privilegies because it needs to watch the created and forked processes in the system in order to detect when a `java` process is launched to then inspect if it was the game, to consider killing the launcher running in the background to save up to 300 MB of RAM. This is a limitation of the Linux kernel API for process watching and/or the `cnproc` crate we are using as a dependency. The real launcher and the game (`java` process) and all other tasks that can run on the user context, do so, and only the parts requiring root context are ran on the root context. This ensures your security is not compromised.

The launcher already tries to elevate to the sudo / root context on launch before starting watching process creation via `cnproc`. For this to work when not launching from a terminal without having to enter your password each time, you must edit your sudo config to add the launcher binary as not requiring password, follow the below steps for it:
 - Run `sudo EDITOR=gnome-text-editor visudo` (replace with your graphical editor; or nano/vim/vi terminal editor for what you prefer)
 - Scroll to the bottom, make a new line after `@includedir /etc/sudoers.d`
 - Write `yourusername ALL = (root) NOPASSWD: /usr/bin/minecraft-launcher`, replace yourusername with your user name obviously.
 - Save the file and quit
 - Since we used `visudo`, it will syntax check the file so that if you make any errors you are not locked out of the `sudo` command.

# Recommended Launcher Settings

# Must have
`Keep the Launcher open while games are running` : Unironically required for the automatic killing of the launcher to work. If this is not enabled, the launcher UI disappears and only generic CEF and background processes are left which we can't be 100% sure if they belong to launcher due to parent process disappearing. With it on, the launcher wrapper will correctly kill the launcher after launching the game safely to save resources.

# Recommended
- Do not use beta version of the launcher, while it never broke this project, Microsoft ships broken versions of the real launcher itself frequently, locking you out of launching old versions of the game for example. These take them days to fix, so you better use the stable version, as you can't downgrade once you enable the beta launcher one time either without uninstalling.
- Keep `Open output log when Minecraft: Java Edition starts` off.
- Keep `Automatically send Minecraft: Java Edition crash reports to Mojang Studios` off.
- You can configure rest of the not mentioned settings to your liking.

# Upgrading the Launcher

Launcher updates work fine and do not need reinstallation of the wrapper, as Mojang itself uses a wrapper to launch their launcher. You only need to reinstall our wrapper if you update the system level launcher, e.g the minecraft-launcher .deb package itself.

If you updated the .deb package, you must install the wrapper again, but in addition to the install argument, use the upgrade argument as well, e.g, `--install --upgrade`.

# Installation

- Download the latest binary from GitHub Actions.
- Open your terminal and run the program with the `--install` argument.
- Enter sudo password if prompted.
- The launcher will be copied to `/usr/bin/minecraft-launcher`, replacing the old one. The old launcher will be copied to `/usr/bin/minecraft-launcher-real`.
- You can repeat this process to install newer versions of our wrapper. For upgrading the .deb launcher, you must run with `--install --upgrade` afterwards like mentioned above.
- The program will automatically detect if it's installed already.

# Additional Tweaks

While we launch the launcher with the nice priority of -6 and the game (the `java`) process inherits it, and lowering niceness value is always allowed, and java's `Thread.MAX_PRIORITY` maps to -5 so in theory java code should be able to tweak thread priorities further by then, this might not work on some systems with stricter security.

You should enable the capability `CAP_SYS_NICE` for the `java` binary in that case.

Keep in mind you can only do this for .deb installed java packages, as doing it to java binaries inside unzipped folder java packages will break them due to `LD_LIBRARY_PATH` to load `libjli.so` failing due to the extra capability disallowing dynamic linking for security.

Use the command `sudo setcap cap_sys_nice=ep /usr/lib/jvm/java-21-openjdk-amd64/bin/java` to grant the `CAP_SYS_NICE` capability, replacing the path with your system java path that you use to launch the game. `=ep` means Effective and Permitted. You could also use `=eip` for Effective, Inheritable and Permitted but it will not do much as the game does not start new processes with the special `execve` to inherit the capability.

You must also launch the game with `-XX:ThreadPriorityPolicy=1` JVM argument when on Linux so that the JVM actually sets the priorities. You might get a warning from JVM that it requires root user, but `CAP_SYS_NICE` works just fine.

# Additional Resources

A user-space thread priority tweaking daemon will improve your performance if you use our wrapper and/or followed the `CAP_SYS_NICE` guide. This is should be done inside user-context java code by a mod.

For such a mod that also provides other performance enhancing features, see my other project [DarkUtils](https://github.com/TheDGOfficial/DarkUtils). The thread priority tweaker code can be found at [this file](https://github.com/TheDGOfficial/DarkUtils/blob/main/src/main/java/gg/darkutils/feat/performance/ThreadPriorityTweaker.java).

# Recommended JVM Flags

Note: You should understand all of the flags before copy and pasting this.

You MUST change the libopenal.so and libglfw.so versions in the arguments if your distro ships a version other than mine, which is at the moment GLFW 3.4 and OpenAL 1.24.2.

`-Xss512k -Xms768m -XX:SoftMaxHeapSize=1536m -Xmx4g -XX:+UseZGC -XX:+DisableExplicitGC -XX:+UseCompactObjectHeaders -XX:+UseStringDeduplication -XX:+PerfDisableSharedMem -XX:-UsePerfData -XX:+DisableAttachMechanism -XX:MaxDirectMemorySize=2g -XX:-DontCompileHugeMethods -XX:MaxInlineLevel=40 -XX:TypeProfileMajorReceiverPercent=30 -XX:MaxNodeLimit=240000 -XX:NodeLimitFudgeFactor=8000 -XX:AllocatePrefetchStyle=3 -XX:-JavaMonitorsInStackTrace -XX:+AllowParallelDefineClass -XX:NmethodSweepActivity=1 -Dsun.rmi.dgc.client.gcInterval=2147483647 -Dsun.rmi.dgc.server.gcInterval=2147483647 -Dsun.rmi.transport.tcp.maxConnectionThreads=0 -Dsun.awt.enableExtraMouseButtons=false -Dsun.awt.disablegrab=true -Djdk.nio.maxCachedBufferSize=262144 -Djdk.gtk.version=3 -Djdk.util.jar.enableMultiRelease=force -Djava.net.preferIPv4Stack=true -Djava.rmi.server.randomIDs=false -Duser.language=en -Duser.country=US -Dio.netty.maxDirectMemory=2147483648 -Dorg.lwjgl.util.NoChecks=true -Dorg.lwjgl.util.NoFunctionChecks=true -Dorg.lwjgl.glfw.libname=/usr/lib/x86_64-linux-gnu/libglfw.so.3.4 -Dorg.lwjgl.openal.libname=/usr/lib/x86_64-linux-gnu/libopenal.so.1.24.2 -Dintel.driver.cmdline=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe --enable-preview --enable-native-access=ALL-UNNAMED -XX:ThreadPriorityPolicy=1 -XX:+UnlockExperimentalVMOptions -XX:+TrustFinalNonStaticFields -Xlog:async --add-modules jdk.incubator.vector`

- `-Xss512k` : Reduces thread stack size, which will reduce memory usage when lots of threads are live.
- `-Xms768m` : Starts the game with a minimal heap while also providing fast warmup. You can also use `-Xms2m` or `-Xms4m` but it will be a bit slower warmup. Read the JEP for [G1GC](https://openjdk.org/jeps/8359211) and [ZGC](https://openjdk.org/jeps/8377305) about this for more details. There's also a way to set initial and minimum heap size seperately, see [here](https://bugs.openjdk.org/browse/JDK-8223837), but we do not use it as Minecraft always uses around 512 MB to 800 MB of memory even when idle with memory optimization mods like FerriteCore, so it is better to allocate that much memory and enforce it as the minimum memory, since realistically the app will never use any less memory.
- `-XX:SoftMaxHeapSize=1536m` : Makes ZGC use at most this much memory when not under pressure. If the game strictly needs more memory it can grow further up to the `-Xmx` value.
- `-Xmx4g` : This the maximum amount of memory used for heap. ZGC will try to do proactive and aggressive GC if heap gets above this size. Note that this does not include memory used by the JVM itself nor the native/off-heap memory used by e.g., Netty, so you should not set this too high nor equal to `-Xms` or `-XX:SoftMaxHeapSize`, that is for servers where only one app is ran. For desktops, it is better to only let the game use minimal memory it needs, which allows memory to be used by other apps and even the OS itself for filesystem caching, which in turn will make I/O operations much faster.
- `-XX:+UseZGC` : This turns on the Z Garbage Collector which is the newest and lowest-latency garbage collector. For servers, the good old battle tested `G1GC` might be better for throughput, but it is horrible for client with its high latency, it skews the frame times and 1%/0.1% lows due to the occassional frame taking 2x or more time because of GC. ZGC ensures good tail latency.
- `-XX:+DisableExplicitGC` : Vanilla Minecraft calls `System.gc()` a lot which is a bad programming practice. It calls it especially after switching worlds. While this often holds up and releases memory immediately in singleplayer to unload the old world for example when leaving the singleplayer world or going to the nether, it makes no sense on a BungeeCord server due to the constant world/server switch and the need to load the same map. The GC initiated by `System.gc()` is not guaranteed to be concurrent and it is not even guranteed to be ran either, so this argument disables it. Do note that this also disables the game's `OutOfMemoryError` (which calls `System.gc()`), but that usually never works anyway and the JVM proceeds to generate a heapdump because the JVM only triggers the `OutOfMemoryError` when GC was tried and seemd to not relieve the memory pressure already (`UseGCOverheadLimit` and `GCTimeRatio` flags).
- `-XX:+UseCompactObjectHeaders` : This enables a compact representation of objects which saves up to 20% memory.
- `-XX:+UseStringDeduplication` : A similar story, shares strings that survived at least 3 GC cycles to deduplicate them in memory, often saving up to 20% memory as well.
- `-XX:+PerfDisableSharedMem` : Disables writing shared performance counters to the disk to avoid large GC mmap pauses. [Source](https://www.evanjones.ca/jvm-mmap-pause.html)
- `-XX:-UsePerfData -XX:+DisableAttachMechanism -Dsun.rmi.transport.tcp.maxConnectionThreads=0 -Djava.rmi.server.randomIDs=false` : This further disables connecting any external profiling or monitoring tool. In-app profilers like Spark can still self attach.
- `-XX:MaxDirectMemorySize=2g` : Puts a lower memory limit to the native memory used by direct byte buffers than the default which is double the max heap size.
- `-XX:-DontCompileHugeMethods` : The JVM skips compiling any method above 8000 bytes in bytecode size by default, which results in poor performance when running those methods since they aren't even C1 compiled let alone C2 compiled, they are always interpreted. This disables it so that all methods are eligible for C2 compilation. This makes the steady state/peak performance better.
- `-XX:MaxNodeLimit=240000 -XX:NodeLimitFudgeFactor=8000 -XX:AllocatePrefetchStyle=3 -XX:NmethodSweepActivity=1` : Improves performance related to code cache and allocation prefetching. [Source](https://github.com/brucethemoose/Minecraft-Performance-Flags-Benchmarks)
- `-XX:-JavaMonitorsInStackTrace` : Minecraft creates lots of exceptions which has to generate the stack trace even if unused. While JVM might optimize this (`OmitStackInFastThrow`), it often does not. This option removes Java Monitors from stack traces which decreases the size of stack traces.
- `-XX:+AllowParallelDefineClass` : The JDK API provides a way to declare Class Loaders as parallel-capable, but classes are never actually loaded on parallel even for parallel-capable class loaders. This option enables parallel class loading to fasten the game startup if your mod loader's class loader is parallel-capable.
- `-Dsun.rmi.dgc.client.gcInterval=2147483647 -Dsun.rmi.dgc.server.gcInterval=2147483647` : Prevents RMI periodic GC.
- `-Dsun.awt.enableExtraMouseButtons=false -Dsun.awt.disablegrab=true -Djdk.gtk.version=3` : AWT related fixes, often only affects early loading screen or other mods that show an AWT window before the LWJGL window.
- `-Djdk.nio.maxCachedBufferSize=262144` : Fixes a memory leak where cached buffers are never released and can grow over the heap size and even the direct memory size. [Source](https://www.evanjones.ca/java-bytebuffer-leak.html)
- `-Djdk.util.jar.enableMultiRelease=force` : Fixes shaded / fat jars ignoring loading the newer versions of classes for the current Java runtime due to missing `Multi-Release: true` entry in `MANIFEST.MF`, as a result of e.g. gradle shadow or maven shade plugin.
- `-Djava.net.preferIPv4Stack=true` : Makes Java always prefer IPv4 adresses, which reduces latency when fallbacking from IPv6 takes too long. Also fixes a warning in logs related to LAN coming from vanilla: "Unable to start LAN server detection: Network interface not configured for IPv4", which mostly makes LAN unusable. With the argument, it will be fixed.
- `-Duser.language=en -Duser.country=US` : Fixes bad code using .toLowerCase or .toUpperCase on strings without specifying `Locale.ROOT` as an argument to work on systems that use other languages.
- `-Dio.netty.maxDirectMemory=2147483648` : Does the same thing as `MaxDirectMemorySize`, but puts a limit on the Netty side before JVM-side, which prevents `OutOfMemoryError`s as netty will try to reclaim the memory before JVM does its own error handling.
- `-Dorg.lwjgl.util.NoChecks=true -Dorg.lwjgl.util.NoFunctionChecks=true` : Disables some runtime checks done by lwjgl to improve performance. This documented as "can be disabled in production to improve performance" in their own source code.
- `-Dorg.lwjgl.glfw.libname=/usr/lib/x86_64-linux-gnu/libglfw.so.3.4 -Dorg.lwjgl.openal.libname=/usr/lib/x86_64-linux-gnu/libopenal.so.1.24.2` : Makes the game use system provided GLFW and OpenAL, which is often newer than what Minecraft ships by default, and less buggier since it integrates with other system components. It also allows using of the `Wayland` GLFW backend.
- `-Dintel.driver.cmdline=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe` : Mojang uses this argument to not break intel driver optimizations targeting Minecraft when running in an integrated intel GPU. Does no harm for other GPU vendors, safe to keep.
- `--enable-preview` : Enables use of the preview features such as `StableValues` or `LazyConstants` JEPs, which might improve performance if you have some mods utilizing them.
- `--enable-native-access=ALL-UNNAMED` : Makes Netty use a direct `malloc` and `free` functions over using the JDK Cleaner API, which is both faster and more memory efficient. Note that using io.netty.common instead of ALL-UNNAMED does not work as Minecraft loads Netty in a way that makes it's module-info.class not registered by the JVM as a module. You will get an unknown module warning if you launch with io.netty.common.
- `-XX:ThreadPriorityPolicy=1` : Enables calling OS methods in Linux for setting thread priorities, by default this not the case and priority change requests are silently discarded, which is unfortunate. This flag fixes it. Note that you need `CAP_SYS_NICE` on the java binary from previous steps for the priority changes to actually succeed.
- `-XX:+UnlockExperimentalVMOptions -XX:+TrustFinalNonStaticFields` : Enables an experimental but performance-benefiting option that does not seem to cause any issues. By default, Java only trusts static final fields for constant folding, and some JDK-internal `@Stable` annotated non-static final fields. This flag makes the JVM able to constant fold non-static final fields in application code as well, which improves performance. Note that, modifying such final fields are discouraged in Java 26 onward anyways, so this should be safe. See [JEP 500](https://openjdk.org/jeps/500) for info on what's changing regarding modifying final fields via reflection in the JVM.
- `-XX:MaxInlineLevel=40 -XX:TypeProfileMajorReceiverPercent=30` : Recommended by Intel for more Java JIT performance gains. See https://github.com/intel/optimization-zone/blob/main/software/java/configuration-optimizations.md#tuning-the-jit-compiler-forcing-compilation--inlining for details.
- `-Xlog:async` : Makes JVM-printed messages such as GC Logging, JIT logging, HotSpot warnings, etc., use async logging. See https://bugs.openjdk.org/browse/JDK-8264323
- `--add-modules jdk.incubator.vector` : Enables the use of optimized Vector module for SIMD. You will need a mod to take advantage of this.
