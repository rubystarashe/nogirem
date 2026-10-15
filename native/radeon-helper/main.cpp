#include <cstdio>
#include <iostream>
#include <sstream>
#include <string>
#include <vector>
#include <windows.h>
#include "ADLXHelper.h"
#include "I3DSettings.h"
#include "ISystem.h"

using namespace adlx;

struct GoalState
{
    std::string key;
    std::string label;
    bool supported = false;
    bool met = false;
    std::string currentValue = "지원 안 함";
};

struct GpuSnapshot
{
    IADLXGPU* gpu = nullptr;
    bool chillSupported = false;
    bool chillEnabled = false;
    bool enhancedSyncSupported = false;
    bool enhancedSyncEnabled = false;
    bool antiLagSupported = false;
    bool antiLagEnabled = false;
    bool verticalSyncSupported = false;
    ADLX_WAIT_FOR_VERTICAL_REFRESH_MODE verticalSyncMode = WFVR_ALWAYS_ON;
};

template <typename T>
void releaseInterface(T*& value)
{
    if (value != nullptr)
    {
        value->Release();
        value = nullptr;
    }
}

std::string escapeJson(const std::string& value)
{
    std::ostringstream output;
    for (const unsigned char character : value)
    {
        switch (character)
        {
        case '"': output << "\\\""; break;
        case '\\': output << "\\\\"; break;
        case '\b': output << "\\b"; break;
        case '\f': output << "\\f"; break;
        case '\n': output << "\\n"; break;
        case '\r': output << "\\r"; break;
        case '\t': output << "\\t"; break;
        default:
            if (character < 0x20)
            {
                const char* hex = "0123456789abcdef";
                output << "\\u00" << hex[(character >> 4) & 0x0f] << hex[character & 0x0f];
            }
            else
            {
                output << character;
            }
        }
    }
    return output.str();
}

void mergeGoal(GoalState& aggregate, bool supported, bool met, const std::string& currentValue)
{
    if (!supported) return;
    if (!aggregate.supported)
    {
        aggregate.supported = true;
        aggregate.met = true;
        aggregate.currentValue = currentValue;
    }
    aggregate.met = aggregate.met && met;
    if (aggregate.currentValue != currentValue) aggregate.currentValue = "GPU별 설정 다름";
}

std::string booleanText(bool enabled)
{
    return enabled ? "켜기" : "끄기";
}

bool readBooleanSetting(
    IADLX3DSettingsServices* settings,
    IADLXGPU* gpu,
    const std::string& key,
    bool apply,
    bool target,
    bool& supported,
    bool& enabled,
    std::string& error)
{
    ADLX_RESULT result = ADLX_FAIL;
    adlx_bool isSupported = false;
    adlx_bool isEnabled = false;

    if (key == "antiLag")
    {
        IADLX3DAntiLag* setting = nullptr;
        result = settings->GetAntiLag(gpu, &setting);
        if (ADLX_SUCCEEDED(result) && setting != nullptr)
        {
            result = setting->IsSupported(&isSupported);
            if (ADLX_SUCCEEDED(result) && isSupported && apply) result = setting->SetEnabled(target);
            if (ADLX_SUCCEEDED(result) && isSupported) result = setting->IsEnabled(&isEnabled);
            releaseInterface(setting);
        }
    }
    else if (key == "chill")
    {
        IADLX3DChill* setting = nullptr;
        result = settings->GetChill(gpu, &setting);
        if (ADLX_SUCCEEDED(result) && setting != nullptr)
        {
            result = setting->IsSupported(&isSupported);
            if (ADLX_SUCCEEDED(result) && isSupported && apply) result = setting->SetEnabled(target);
            if (ADLX_SUCCEEDED(result) && isSupported) result = setting->IsEnabled(&isEnabled);
            releaseInterface(setting);
        }
    }
    else
    {
        IADLX3DEnhancedSync* setting = nullptr;
        result = settings->GetEnhancedSync(gpu, &setting);
        if (ADLX_SUCCEEDED(result) && setting != nullptr)
        {
            result = setting->IsSupported(&isSupported);
            if (ADLX_SUCCEEDED(result) && isSupported && apply) result = setting->SetEnabled(target);
            if (ADLX_SUCCEEDED(result) && isSupported) result = setting->IsEnabled(&isEnabled);
            releaseInterface(setting);
        }
    }

    if (!ADLX_SUCCEEDED(result))
    {
        error = key + " 설정 처리 실패: ADLX " + std::to_string(result);
        return false;
    }
    supported = isSupported;
    enabled = isEnabled;
    return true;
}

bool readVerticalSync(
    IADLX3DSettingsServices* settings,
    IADLXGPU* gpu,
    bool apply,
    ADLX_WAIT_FOR_VERTICAL_REFRESH_MODE target,
    bool& supported,
    ADLX_WAIT_FOR_VERTICAL_REFRESH_MODE& mode,
    std::string& error)
{
    IADLX3DWaitForVerticalRefresh* setting = nullptr;
    ADLX_RESULT result = settings->GetWaitForVerticalRefresh(gpu, &setting);
    adlx_bool isSupported = false;
    if (ADLX_SUCCEEDED(result) && setting != nullptr)
    {
        result = setting->IsSupported(&isSupported);
        if (ADLX_SUCCEEDED(result) && isSupported && apply) result = setting->SetMode(target);
        if (ADLX_SUCCEEDED(result) && isSupported) result = setting->GetMode(&mode);
        releaseInterface(setting);
    }
    if (!ADLX_SUCCEEDED(result))
    {
        error = "수직 동기화 설정 처리 실패: ADLX " + std::to_string(result);
        return false;
    }
    supported = isSupported;
    return true;
}

bool readSnapshot(IADLX3DSettingsServices* settings, GpuSnapshot& snapshot, std::string& error)
{
    if (!readBooleanSetting(settings, snapshot.gpu, "chill", false, false, snapshot.chillSupported, snapshot.chillEnabled, error)) return false;
    if (!readBooleanSetting(settings, snapshot.gpu, "enhancedSync", false, false, snapshot.enhancedSyncSupported, snapshot.enhancedSyncEnabled, error)) return false;
    if (!readVerticalSync(settings, snapshot.gpu, false, WFVR_ALWAYS_OFF, snapshot.verticalSyncSupported, snapshot.verticalSyncMode, error)) return false;
    return readBooleanSetting(settings, snapshot.gpu, "antiLag", false, true, snapshot.antiLagSupported, snapshot.antiLagEnabled, error);
}

bool writeTargets(
    IADLX3DSettingsServices* settings,
    const GpuSnapshot& snapshot,
    bool restore,
    std::string& error)
{
    bool supported = false;
    bool enabled = false;
    ADLX_WAIT_FOR_VERTICAL_REFRESH_MODE mode = WFVR_ALWAYS_ON;
    bool success = true;
    std::string settingError;
    const bool chillTarget = restore ? snapshot.chillEnabled : false;
    settingError.clear();
    if (!readBooleanSetting(settings, snapshot.gpu, "chill", true, chillTarget, supported, enabled, settingError)
        || (supported && enabled != chillTarget))
    {
        if (settingError.empty()) settingError = "Radeon Chill 설정 검증 실패";
        error = settingError;
        if (!restore) return false;
        success = false;
    }
    const bool enhancedSyncTarget = restore ? snapshot.enhancedSyncEnabled : false;
    settingError.clear();
    if (!readBooleanSetting(settings, snapshot.gpu, "enhancedSync", true, enhancedSyncTarget, supported, enabled, settingError)
        || (supported && enabled != enhancedSyncTarget))
    {
        if (settingError.empty()) settingError = "Enhanced Sync 설정 검증 실패";
        if (error.empty()) error = settingError;
        if (!restore) return false;
        success = false;
    }
    const ADLX_WAIT_FOR_VERTICAL_REFRESH_MODE verticalSyncTarget = restore ? snapshot.verticalSyncMode : WFVR_ALWAYS_OFF;
    settingError.clear();
    if (!readVerticalSync(settings, snapshot.gpu, true, verticalSyncTarget, supported, mode, settingError)
        || (supported && mode != verticalSyncTarget))
    {
        if (settingError.empty()) settingError = "수직 동기화 설정 검증 실패";
        if (error.empty()) error = settingError;
        if (!restore) return false;
        success = false;
    }
    const bool antiLagTarget = restore ? snapshot.antiLagEnabled : true;
    settingError.clear();
    if (!readBooleanSetting(settings, snapshot.gpu, "antiLag", true, antiLagTarget, supported, enabled, settingError)
        || (supported && enabled != antiLagTarget))
    {
        if (settingError.empty()) settingError = "Radeon Anti-Lag 설정 검증 실패";
        if (error.empty()) error = settingError;
        if (!restore) return false;
        success = false;
    }
    return success;
}

std::string verticalSyncText(ADLX_WAIT_FOR_VERTICAL_REFRESH_MODE mode)
{
    switch (mode)
    {
    case WFVR_ALWAYS_OFF: return "항상 끄기";
    case WFVR_OFF_UNLESS_APP_SPECIFIES: return "응용 프로그램 지정 시 켜기";
    case WFVR_ON_UNLESS_APP_SPECIFIES: return "응용 프로그램 지정 시 끄기";
    case WFVR_ALWAYS_ON: return "항상 켜기";
    default: return "알 수 없음";
    }
}

void printResult(
    bool detected,
    const std::vector<std::string>& gpuNames,
    const std::vector<GoalState>& goals,
    const std::string& reason)
{
    bool allMet = detected;
    bool hasSupportedGoal = false;
    for (const GoalState& goal : goals)
    {
        if (!goal.supported) continue;
        hasSupportedGoal = true;
        allMet = allMet && goal.met;
    }
    allMet = allMet && hasSupportedGoal;

    std::ostringstream output;
    output << "{\"detected\":" << (detected ? "true" : "false") << ",\"gpus\":[";
    for (size_t index = 0; index < gpuNames.size(); ++index)
    {
        if (index > 0) output << ",";
        output << "{\"name\":\"" << escapeJson(gpuNames[index]) << "\"}";
    }
    output << "],\"goals\":[";
    for (size_t index = 0; index < goals.size(); ++index)
    {
        if (index > 0) output << ",";
        const GoalState& goal = goals[index];
        output
            << "{\"key\":\"" << goal.key
            << "\",\"label\":\"" << escapeJson(goal.label)
            << "\",\"supported\":" << (goal.supported ? "true" : "false")
            << ",\"met\":" << (goal.met ? "true" : "false")
            << ",\"currentValue\":\"" << escapeJson(goal.currentValue) << "\"}";
    }
    output << "],\"allMet\":" << (allMet ? "true" : "false")
           << ",\"tuningAccessed\":false";
    if (reason.empty()) output << ",\"reason\":null";
    else output << ",\"reason\":\"" << escapeJson(reason) << "\"";
    output << "}";
    std::cout << output.str() << std::endl;
}

int runHelper(int argc, char** argv)
{
    bool apply = false;
    for (int index = 1; index < argc; ++index)
    {
        if (std::string(argv[index]) == "--apply") apply = true;
    }

    std::vector<GoalState> goals = {
        {"verticalSyncOff", "수직 동기화 항상 끄기"},
        {"enhancedSyncOff", "Enhanced Sync 끄기"},
        {"antiLagOn", "Radeon Anti-Lag 켜기"},
        {"chillOff", "Radeon Chill 끄기"},
    };
    std::vector<std::string> gpuNames;
    std::string error;

    // 사용자 전력·전압 프로필과 분리된 3D 설정 서비스만 사용합니다.
    ADLX_RESULT result = g_ADLX.Initialize();
    if (!ADLX_SUCCEEDED(result))
    {
        printResult(false, gpuNames, goals, "AMD ADLX 초기화 실패: " + std::to_string(result));
        return 0;
    }

    IADLXSystem* system = g_ADLX.GetSystemServices();
    IADLXGPUList* gpus = nullptr;
    IADLX3DSettingsServices* settings = nullptr;
    result = system == nullptr ? ADLX_FAIL : system->GetGPUs(&gpus);
    if (ADLX_SUCCEEDED(result)) result = system->Get3DSettingsServices(&settings);
    if (!ADLX_SUCCEEDED(result) || gpus == nullptr || settings == nullptr)
    {
        releaseInterface(settings);
        releaseInterface(gpus);
        g_ADLX.Terminate();
        printResult(false, gpuNames, goals, "AMD GPU 설정 서비스를 찾지 못했습니다");
        return 0;
    }

    std::vector<GpuSnapshot> snapshots;
    for (adlx_uint index = 0; index < gpus->Size(); ++index)
    {
        IADLXGPU* gpu = nullptr;
        if (!ADLX_SUCCEEDED(gpus->At(index, &gpu)) || gpu == nullptr) continue;
        const char* name = nullptr;
        if (ADLX_SUCCEEDED(gpu->Name(&name)) && name != nullptr) gpuNames.emplace_back(name);
        else gpuNames.emplace_back("AMD Radeon GPU");

        GpuSnapshot snapshot;
        snapshot.gpu = gpu;
        std::string settingError;
        if (!readSnapshot(settings, snapshot, settingError) && error.empty()) error = settingError;
        snapshots.push_back(snapshot);
    }

    if (apply && error.empty())
    {
        for (const GpuSnapshot& snapshot : snapshots)
        {
            std::string settingError;
            if (!writeTargets(settings, snapshot, false, settingError))
            {
                error = settingError;
                break;
            }
        }
        if (!error.empty())
        {
            std::string rollbackError;
            for (const GpuSnapshot& snapshot : snapshots)
            {
                std::string settingError;
                if (!writeTargets(settings, snapshot, true, settingError) && rollbackError.empty())
                    rollbackError = settingError;
            }
            if (!rollbackError.empty()) error += " / 이전 3D 설정 복원 실패: " + rollbackError;
            else error += " / 이전 3D 설정을 복원했습니다";
        }
    }

    for (const GpuSnapshot& snapshot : snapshots)
    {
        bool supported = false;
        bool enabled = false;
        ADLX_WAIT_FOR_VERTICAL_REFRESH_MODE mode = WFVR_ALWAYS_ON;
        std::string settingError;

        if (readBooleanSetting(settings, snapshot.gpu, "chill", false, false, supported, enabled, settingError))
            mergeGoal(goals[3], supported, !enabled, booleanText(enabled));
        else if (error.empty()) error = settingError;

        if (readBooleanSetting(settings, snapshot.gpu, "enhancedSync", false, false, supported, enabled, settingError))
            mergeGoal(goals[1], supported, !enabled, booleanText(enabled));
        else if (error.empty()) error = settingError;

        if (readVerticalSync(settings, snapshot.gpu, false, WFVR_ALWAYS_OFF, supported, mode, settingError))
            mergeGoal(goals[0], supported, mode == WFVR_ALWAYS_OFF, verticalSyncText(mode));
        else if (error.empty()) error = settingError;

        if (readBooleanSetting(settings, snapshot.gpu, "antiLag", false, true, supported, enabled, settingError))
            mergeGoal(goals[2], supported, enabled, booleanText(enabled));
        else if (error.empty()) error = settingError;

        IADLXGPU* gpu = snapshot.gpu;
        releaseInterface(gpu);
    }

    releaseInterface(settings);
    releaseInterface(gpus);
    g_ADLX.Terminate();
    printResult(!gpuNames.empty(), gpuNames, goals, error);
    return 0;
}

int main(int argc, char** argv)
{
    SetConsoleOutputCP(CP_UTF8);
    int exitCode = 0;
    __try
    {
        exitCode = runHelper(argc, argv);
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        std::printf(
            "{\"detected\":false,\"gpus\":[],\"goals\":[],\"allMet\":false,"
            "\"tuningAccessed\":false,"
            "\"reason\":\"AMD \\ub4dc\\ub77c\\uc774\\ubc84 \\uc124\\uc815 API "
            "\\uc2e4\\ud589 \\uc911 \\uc624\\ub958\\uac00 \\ubc1c\\uc0dd"
            "\\ud588\\uc2b5\\ub2c8\\ub2e4 (0x%08lX)\"}\n",
            GetExceptionCode());
        exitCode = 0;
    }
    std::cout.flush();
    std::cerr.flush();
    std::fflush(nullptr);
    ExitProcess(static_cast<UINT>(exitCode));
}
