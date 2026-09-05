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
        if (ADLX_SUCCEEDED(result) && isSupported && apply) result = setting->SetMode(WFVR_ALWAYS_OFF);
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
    output << "],\"allMet\":" << (allMet ? "true" : "false");
    if (reason.empty()) output << ",\"reason\":null";
    else output << ",\"reason\":\"" << escapeJson(reason) << "\"";
    output << "}";
    std::cout << output.str() << std::endl;
}

int main(int argc, char** argv)
{
    SetConsoleOutputCP(CP_UTF8);
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

    for (adlx_uint index = 0; index < gpus->Size(); ++index)
    {
        IADLXGPU* gpu = nullptr;
        if (!ADLX_SUCCEEDED(gpus->At(index, &gpu)) || gpu == nullptr) continue;
        const char* name = nullptr;
        if (ADLX_SUCCEEDED(gpu->Name(&name)) && name != nullptr) gpuNames.emplace_back(name);
        else gpuNames.emplace_back("AMD Radeon GPU");

        bool supported = false;
        bool enabled = false;
        ADLX_WAIT_FOR_VERTICAL_REFRESH_MODE mode = WFVR_ALWAYS_ON;
        std::string settingError;

        if (readBooleanSetting(settings, gpu, "chill", apply, false, supported, enabled, settingError))
            mergeGoal(goals[3], supported, !enabled, booleanText(enabled));
        else if (error.empty()) error = settingError;

        if (readBooleanSetting(settings, gpu, "enhancedSync", apply, false, supported, enabled, settingError))
            mergeGoal(goals[1], supported, !enabled, booleanText(enabled));
        else if (error.empty()) error = settingError;

        if (readVerticalSync(settings, gpu, apply, supported, mode, settingError))
            mergeGoal(goals[0], supported, mode == WFVR_ALWAYS_OFF, verticalSyncText(mode));
        else if (error.empty()) error = settingError;

        if (readBooleanSetting(settings, gpu, "antiLag", apply, true, supported, enabled, settingError))
            mergeGoal(goals[2], supported, enabled, booleanText(enabled));
        else if (error.empty()) error = settingError;

        releaseInterface(gpu);
    }

    releaseInterface(settings);
    releaseInterface(gpus);
    g_ADLX.Terminate();
    printResult(!gpuNames.empty(), gpuNames, goals, error);
    return 0;
}
