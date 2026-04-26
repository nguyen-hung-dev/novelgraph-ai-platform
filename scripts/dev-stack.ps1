[CmdletBinding()]
param(
    [int]$BackendPort = 3000,
    [int]$FrontendPort = 5173,
    [int]$PortSearchSpan = 20,
    [string]$BackendHost = "127.0.0.1",
    [string]$FrontendHost = "127.0.0.1",
    [switch]$DryRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$script:repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$script:jobHandle = [System.IntPtr]::Zero
$script:startedProcesses = New-Object System.Collections.Generic.List[System.Diagnostics.Process]
$script:cleanupDone = $false

function Write-StackInfo {
    param([string]$Message)
    Write-Host "[stack] $Message" -ForegroundColor Cyan
}

function Write-StackWarn {
    param([string]$Message)
    Write-Host "[stack] $Message" -ForegroundColor Yellow
}

function Write-StackError {
    param([string]$Message)
    Write-Host "[stack] $Message" -ForegroundColor Red
}

function Test-CommandAvailable {
    param([string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Ensure-CommandAvailable {
    param([string]$Name)
    if (-not (Test-CommandAvailable -Name $Name)) {
        throw "Required command not found in PATH: $Name"
    }
}

function Add-JobObjectSupport {
    if ("NovelGraph.DevStack.JobObject" -as [type]) {
        return
    }

    Add-Type -TypeDefinition @"
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

namespace NovelGraph.DevStack
{
    internal static class NativeMethods
    {
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        internal static extern IntPtr CreateJobObject(IntPtr lpJobAttributes, string lpName);

        [DllImport("kernel32.dll", SetLastError = true)]
        internal static extern bool SetInformationJobObject(
            IntPtr hJob,
            int jobObjectInfoClass,
            IntPtr lpJobObjectInfo,
            uint cbJobObjectInfoLength
        );

        [DllImport("kernel32.dll", SetLastError = true)]
        internal static extern bool AssignProcessToJobObject(IntPtr hJob, IntPtr hProcess);

        [DllImport("kernel32.dll", SetLastError = true)]
        internal static extern bool CloseHandle(IntPtr handle);
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct JOBOBJECT_BASIC_LIMIT_INFORMATION
    {
        public long PerProcessUserTimeLimit;
        public long PerJobUserTimeLimit;
        public uint LimitFlags;
        public UIntPtr MinimumWorkingSetSize;
        public UIntPtr MaximumWorkingSetSize;
        public uint ActiveProcessLimit;
        public IntPtr Affinity;
        public uint PriorityClass;
        public uint SchedulingClass;
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct IO_COUNTERS
    {
        public ulong ReadOperationCount;
        public ulong WriteOperationCount;
        public ulong OtherOperationCount;
        public ulong ReadTransferCount;
        public ulong WriteTransferCount;
        public ulong OtherTransferCount;
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION
    {
        public JOBOBJECT_BASIC_LIMIT_INFORMATION BasicLimitInformation;
        public IO_COUNTERS IoInfo;
        public UIntPtr ProcessMemoryLimit;
        public UIntPtr JobMemoryLimit;
        public UIntPtr PeakProcessMemoryUsed;
        public UIntPtr PeakJobMemoryUsed;
    }

    public static class JobObject
    {
        private const int JobObjectExtendedLimitInformation = 9;
        private const uint JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000;

        public static IntPtr CreateKillOnClose()
        {
            IntPtr handle = NativeMethods.CreateJobObject(IntPtr.Zero, null);
            if (handle == IntPtr.Zero)
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }

            JOBOBJECT_EXTENDED_LIMIT_INFORMATION info = new JOBOBJECT_EXTENDED_LIMIT_INFORMATION();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            int length = Marshal.SizeOf<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>();
            IntPtr pointer = Marshal.AllocHGlobal(length);

            try
            {
                Marshal.StructureToPtr(info, pointer, false);
                if (!NativeMethods.SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    pointer,
                    (uint)length
                ))
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }

                return handle;
            }
            finally
            {
                Marshal.FreeHGlobal(pointer);
            }
        }

        public static void Assign(IntPtr jobHandle, IntPtr processHandle)
        {
            if (!NativeMethods.AssignProcessToJobObject(jobHandle, processHandle))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
        }

        public static void Close(IntPtr handle)
        {
            if (handle != IntPtr.Zero)
            {
                NativeMethods.CloseHandle(handle);
            }
        }
    }
}
"@
}

function New-KillOnCloseJob {
    Add-JobObjectSupport
    return [NovelGraph.DevStack.JobObject]::CreateKillOnClose()
}

function Close-KillOnCloseJob {
    param([System.IntPtr]$Handle)
    if ($Handle -ne [System.IntPtr]::Zero) {
        [NovelGraph.DevStack.JobObject]::Close($Handle)
    }
}

function Get-ProcessCommandLine {
    param([int]$ProcessId)

    try {
        $process = Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction Stop
        return $process.CommandLine
    }
    catch {
        return $null
    }
}

function Get-PortListenerInfo {
    param([int]$Port)

    $connection = $null

    try {
        $connection = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction Stop |
            Select-Object -First 1
    }
    catch {
        $netstatLine = netstat -ano -p tcp |
            Select-String -Pattern "LISTENING" |
            ForEach-Object { $_.Line } |
            Where-Object {
                if ($_ -match "^\s*TCP\s+\S+:(\d+)\s+\S+\s+LISTENING\s+(\d+)\s*$") {
                    return ([int]$matches[1]) -eq $Port
                }

                return $false
            } |
            Select-Object -First 1

        if ($netstatLine -and $netstatLine -match "^\s*TCP\s+(\S+):(\d+)\s+\S+\s+LISTENING\s+(\d+)\s*$") {
            $connection = [pscustomobject]@{
                LocalAddress  = $matches[1]
                LocalPort     = [int]$matches[2]
                OwningProcess = [int]$matches[3]
            }
        }
    }

    if ($null -eq $connection) {
        return $null
    }

    $process = Get-Process -Id $connection.OwningProcess -ErrorAction SilentlyContinue
    $commandLine = Get-ProcessCommandLine -ProcessId $connection.OwningProcess
    $processPath = $null

    if ($process) {
        try {
            $processPath = $process.Path
        }
        catch {
            $processPath = $null
        }
    }

    return [pscustomobject]@{
        Port        = $Port
        Address     = $connection.LocalAddress
        ProcessId   = $connection.OwningProcess
        ProcessName = $process.ProcessName
        ProcessPath = $processPath
        CommandLine = $commandLine
    }
}

function Test-ManagedRepoProcess {
    param(
        [pscustomobject]$Listener,
        [string[]]$Markers
    )

    if ($null -eq $Listener) {
        return $false
    }

    $repoToken = $script:repoRoot.ToLowerInvariant()
    $haystack = @(
        $Listener.ProcessPath,
        $Listener.CommandLine
    ) -join "`n"

    if ([string]::IsNullOrWhiteSpace($haystack)) {
        return $false
    }

    $normalizedHaystack = $haystack.ToLowerInvariant()
    if (-not $normalizedHaystack.Contains($repoToken)) {
        return $false
    }

    foreach ($marker in $Markers) {
        if ($normalizedHaystack.Contains($marker.ToLowerInvariant())) {
            return $true
        }
    }

    return $false
}

function Stop-ProcessTree {
    param(
        [int]$ProcessId,
        [string]$Reason
    )

    if ($ProcessId -le 0) {
        return
    }

    Write-StackWarn "$Reason. Stopping process tree PID $ProcessId."

    if ($DryRun) {
        return
    }

    & taskkill /PID $ProcessId /T /F *> $null
    Start-Sleep -Milliseconds 800
}

function Resolve-ServicePort {
    param(
        [string]$ServiceName,
        [int]$PreferredPort,
        [string[]]$RestartMarkers
    )

    $listener = Get-PortListenerInfo -Port $PreferredPort
    if ($null -eq $listener) {
        return $PreferredPort
    }

    if (Test-ManagedRepoProcess -Listener $listener -Markers $RestartMarkers) {
        Stop-ProcessTree -ProcessId $listener.ProcessId -Reason "$ServiceName already owned preferred port $PreferredPort"
        return $PreferredPort
    }

    for ($candidate = $PreferredPort + 1; $candidate -le ($PreferredPort + $PortSearchSpan); $candidate++) {
        if ($null -eq (Get-PortListenerInfo -Port $candidate)) {
            Write-StackWarn "$ServiceName preferred port $PreferredPort is busy by PID $($listener.ProcessId). Using port $candidate instead."
            return $candidate
        }
    }

    throw "Unable to find a free port for $ServiceName in range $PreferredPort-$($PreferredPort + $PortSearchSpan)."
}

function Start-ManagedProcess {
    param(
        [string]$Name,
        [string]$Command,
        [hashtable]$Environment
    )

    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = "cmd.exe"
    $psi.Arguments = "/d /c $Command"
    $psi.WorkingDirectory = $script:repoRoot
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $false
    $psi.RedirectStandardError = $false
    $psi.CreateNoWindow = $false

    foreach ($entry in $Environment.GetEnumerator()) {
        $psi.Environment[$entry.Key] = [string]$entry.Value
    }

    Write-StackInfo "Starting $Name with command: $Command"

    if ($DryRun) {
        return $null
    }

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $psi
    $process.EnableRaisingEvents = $true

    $null = $process.Start()
    [NovelGraph.DevStack.JobObject]::Assign($script:jobHandle, $process.Handle)
    $script:startedProcesses.Add($process)
    return $process
}

function Stop-ManagedProcess {
    param([System.Diagnostics.Process]$Process)

    if ($null -eq $Process) {
        return
    }

    if ($Process.HasExited) {
        return
    }

    & taskkill /PID $Process.Id /T /F *> $null
}

function Cleanup-ChildProcesses {
    if ($script:cleanupDone) {
        return
    }

    $script:cleanupDone = $true

    foreach ($process in $script:startedProcesses) {
        Stop-ManagedProcess -Process $process
    }

    Close-KillOnCloseJob -Handle $script:jobHandle
    $script:jobHandle = [System.IntPtr]::Zero
}

try {
    Ensure-CommandAvailable -Name "cargo"
    Ensure-CommandAvailable -Name "pnpm"

    if (-not $DryRun) {
        $script:jobHandle = New-KillOnCloseJob
    }

    $resolvedBackendPort = Resolve-ServicePort -ServiceName "Backend" -PreferredPort $BackendPort -RestartMarkers @(
        "novelgraph-api",
        "cargo run -p novelgraph-api",
        "crates\api"
    )
    $resolvedFrontendPort = Resolve-ServicePort -ServiceName "Frontend" -PreferredPort $FrontendPort -RestartMarkers @(
        "pnpm --filter web dev",
        "apps\web",
        "vite dev"
    )

    $backendUrl = "http://${BackendHost}:$resolvedBackendPort"
    $frontendUrl = "http://${FrontendHost}:$resolvedFrontendPort"

    Write-StackInfo "Repository root: $script:repoRoot"
    Write-StackInfo "Backend URL:  $backendUrl"
    Write-StackInfo "Frontend URL: $frontendUrl"

    $backendEnv = @{
        APP_MODE     = "web"
        HOST         = $BackendHost
        PORT         = $resolvedBackendPort
        RUST_LOG     = "novelgraph_api=info,tower_http=info"
    }
    $frontendEnv = @{
        API_BASE_URL      = $backendUrl
        PUBLIC_API_BASE_URL = $backendUrl
        VITE_API_BASE_URL = $backendUrl
    }

    $backendProcess = Start-ManagedProcess -Name "backend" -Command "cargo run -p novelgraph-api" -Environment $backendEnv
    $frontendProcess = Start-ManagedProcess -Name "frontend" -Command "pnpm --filter web dev -- --host $FrontendHost --port $resolvedFrontendPort" -Environment $frontendEnv

    if ($DryRun) {
        Write-StackInfo "Dry run complete. No process was started."
        return
    }

    Write-StackInfo "Both services are attached to this CLI session. Press Ctrl+C to stop them."

    while ($true) {
        if ($backendProcess.HasExited) {
            throw "Backend exited with code $($backendProcess.ExitCode)."
        }

        if ($frontendProcess.HasExited) {
            throw "Frontend exited with code $($frontendProcess.ExitCode)."
        }

        Start-Sleep -Seconds 1
    }
}
finally {
    Cleanup-ChildProcesses
}
