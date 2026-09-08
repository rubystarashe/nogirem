#include <windows.h>
#include <avrt.h>
#include <audioclient.h>
#include <audioclientactivationparams.h>
#include <comdef.h>
#include <d3d10_1.h>
#include <d3d11.h>
#include <dxgi1_2.h>
#include <mfapi.h>
#include <mfidl.h>
#include <mfreadwrite.h>
#include <mmdeviceapi.h>
#include <propvarutil.h>
#include <shlwapi.h>
#include <wrl.h>
#include <wrl/implements.h>
#include <windows.graphics.capture.interop.h>
#include <windows.graphics.directx.direct3d11.interop.h>
#include <winrt/base.h>
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Graphics.Capture.h>
#include <winrt/Windows.Graphics.DirectX.h>
#include <winrt/Windows.Graphics.DirectX.Direct3D11.h>

#include <algorithm>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <cstring>
#include <cmath>
#include <deque>
#include <filesystem>
#include <fstream>
#include <functional>
#include <iomanip>
#include <iostream>
#include <locale>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <queue>
#include <regex>
#include <sstream>
#include <string>
#include <string_view>
#include <thread>
#include <unordered_map>
#include <vector>

using Microsoft::WRL::ComPtr;
using namespace std::chrono_literals;
using namespace winrt;
using namespace winrt::Windows::Graphics;
using namespace winrt::Windows::Graphics::Capture;
using namespace winrt::Windows::Graphics::DirectX;
using namespace winrt::Windows::Graphics::DirectX::Direct3D11;

namespace fs = std::filesystem;

namespace {

constexpr std::uint64_t Megabyte = 1024ull * 1024ull;
constexpr std::uint64_t Gigabyte = 1024ull * 1024ull * 1024ull;
constexpr std::size_t MaximumQueuedFrames = 3;
constexpr UINT32 AudioSampleRate = 48000;
constexpr UINT32 AudioChannels = 2;
constexpr UINT32 AudioBitsPerSample = 16;
constexpr UINT32 AudioBlockAlignment =
  AudioChannels * AudioBitsPerSample / 8;
constexpr std::size_t MaximumQueuedAudioFrames = AudioSampleRate;
constexpr double AudioTargetPeak = 0.5011872336;
constexpr double AudioLimiterPeak = 0.8912509381;
constexpr double AudioNoiseFloor = 0.0005623413;
constexpr double AudioMaximumGain = 15.8489319246;
constexpr double AudioMinimumGain = 0.25;
constexpr double AudioGainReleaseSeconds = 2.5;

struct Options {
  fs::path statusPath;
  fs::path controlPath;
  fs::path storagePath;
  fs::path gamePath;
  std::wstring codec = L"h264";
  int fps = 60;
  int bitrateMbps = 12;
  int maxHeight = 1080;
  int chunkSeconds = 10;
  int capacityGb = 50;
  int maxDurationSeconds = 3600;
  DWORD parentPid = 0;
};

struct SharedStatus {
  std::mutex mutex;
  bool running = true;
  bool recording = false;
  bool waitingForGame = true;
  bool clipInProgress = false;
  bool audioRecording = false;
  std::wstring codec = L"h264";
  std::wstring error;
  std::wstring audioError;
  std::wstring latestClip;
  double audioGainDb = 0;
  double durationSeconds = 0;
  std::uint64_t bytesUsed = 0;
  std::uint64_t capacityBytes = 0;
  std::atomic_uint64_t droppedFrames = 0;
  std::uint64_t flushCompletedId = 0;
  std::uint64_t clearCompletedId = 0;
  int width = 0;
  int height = 0;
  int fps = 60;
};

class TexturePool {
public:
  ComPtr<ID3D11Texture2D> acquire(
    ID3D11Device* device,
    const D3D11_TEXTURE2D_DESC& description
  ) {
    std::lock_guard lock(mutex_);
    if (
      width_ != description.Width
      || height_ != description.Height
      || format_ != description.Format
    ) {
      available_.clear();
      allocated_ = 0;
      width_ = description.Width;
      height_ = description.Height;
      format_ = description.Format;
    }
    if (!available_.empty()) {
      auto texture = std::move(available_.back());
      available_.pop_back();
      return texture;
    }
    if (allocated_ >= MaximumQueuedFrames) return {};
    ComPtr<ID3D11Texture2D> texture;
    if (FAILED(device->CreateTexture2D(&description, nullptr, &texture))) return {};
    ++allocated_;
    return texture;
  }

  void release(ComPtr<ID3D11Texture2D> texture) {
    if (!texture) return;
    D3D11_TEXTURE2D_DESC description{};
    texture->GetDesc(&description);
    std::lock_guard lock(mutex_);
    if (
      width_ == description.Width
      && height_ == description.Height
      && format_ == description.Format
      && available_.size() < MaximumQueuedFrames
    ) {
      available_.push_back(std::move(texture));
    }
  }

private:
  std::mutex mutex_;
  std::vector<ComPtr<ID3D11Texture2D>> available_;
  std::size_t allocated_ = 0;
  UINT width_ = 0;
  UINT height_ = 0;
  DXGI_FORMAT format_ = DXGI_FORMAT_UNKNOWN;
};

struct FramePacket {
  ComPtr<ID3D11Texture2D> texture;
  std::shared_ptr<TexturePool> texturePool;
  std::chrono::steady_clock::time_point capturedAt;
  int width = 0;
  int height = 0;
};

struct AudioPacket {
  std::vector<std::uint8_t> samples;
  std::chrono::steady_clock::time_point capturedAt;
  UINT32 frameCount = 0;
};

class AudioGainController {
public:
  double process(std::vector<std::uint8_t>& samples, UINT32 frameCount) {
    if (samples.size() < sizeof(std::int16_t) || frameCount == 0) {
      return gainDecibels();
    }
    auto* pcm = reinterpret_cast<std::int16_t*>(samples.data());
    const std::size_t sampleCount = samples.size() / sizeof(std::int16_t);
    int peak = 0;
    for (std::size_t index = 0; index < sampleCount; ++index) {
      peak = std::max(peak, std::abs(static_cast<int>(pcm[index])));
    }
    const double normalizedPeak = static_cast<double>(peak) / 32768.0;
    if (normalizedPeak <= AudioNoiseFloor) return gainDecibels();

    const double desiredGain = std::clamp(
      AudioTargetPeak / normalizedPeak,
      AudioMinimumGain,
      AudioMaximumGain
    );
    if (!initialized_) {
      gain_ = desiredGain;
      initialized_ = true;
    } else if (desiredGain < gain_) {
      gain_ = desiredGain;
    } else {
      const double packetSeconds =
        static_cast<double>(frameCount) / AudioSampleRate;
      const double releaseAmount =
        1.0 - std::exp(-packetSeconds / AudioGainReleaseSeconds);
      gain_ += (desiredGain - gain_) * releaseAmount;
    }

    const int limiter = static_cast<int>(
      std::lround(AudioLimiterPeak * 32767.0)
    );
    for (std::size_t index = 0; index < sampleCount; ++index) {
      const auto amplified = static_cast<int>(
        std::lround(static_cast<double>(pcm[index]) * gain_)
      );
      pcm[index] = static_cast<std::int16_t>(
        std::clamp(amplified, -limiter, limiter)
      );
    }
    return gainDecibels();
  }

private:
  double gainDecibels() const {
    return initialized_ ? 20.0 * std::log10(gain_) : 0.0;
  }

  double gain_ = 1.0;
  bool initialized_ = false;
};

std::int64_t epochMilliseconds() {
  return std::chrono::duration_cast<std::chrono::milliseconds>(
    std::chrono::system_clock::now().time_since_epoch()
  ).count();
}

std::chrono::steady_clock::time_point steadyTimeFromQpc100Nanoseconds(
  UINT64 qpcPosition
) {
  LARGE_INTEGER counter{};
  LARGE_INTEGER frequency{};
  const auto now = std::chrono::steady_clock::now();
  if (
    !QueryPerformanceCounter(&counter)
    || !QueryPerformanceFrequency(&frequency)
    || frequency.QuadPart <= 0
  ) return now;
  const long double currentQpc100Nanoseconds =
    static_cast<long double>(counter.QuadPart)
    * 10000000.0L
    / static_cast<long double>(frequency.QuadPart);
  const long double deltaNanoseconds =
    (static_cast<long double>(qpcPosition) - currentQpc100Nanoseconds) * 100.0L;
  if (std::abs(deltaNanoseconds) > 60.0L * 1000000000.0L) return now;
  return now + std::chrono::nanoseconds(
    static_cast<std::int64_t>(std::llround(deltaNanoseconds))
  );
}

std::wstring lower(std::wstring value) {
  std::transform(value.begin(), value.end(), value.begin(), towlower);
  return value;
}

std::string utf8(const std::wstring& value) {
  if (value.empty()) return {};
  const int size = WideCharToMultiByte(
    CP_UTF8,
    0,
    value.c_str(),
    static_cast<int>(value.size()),
    nullptr,
    0,
    nullptr,
    nullptr
  );
  std::string result(static_cast<std::size_t>(size), '\0');
  WideCharToMultiByte(
    CP_UTF8,
    0,
    value.c_str(),
    static_cast<int>(value.size()),
    result.data(),
    size,
    nullptr,
    nullptr
  );
  return result;
}

std::string jsonEscape(const std::wstring& value) {
  const auto source = utf8(value);
  std::ostringstream output;
  for (const unsigned char character : source) {
    switch (character) {
      case '"': output << "\\\""; break;
      case '\\': output << "\\\\"; break;
      case '\b': output << "\\b"; break;
      case '\f': output << "\\f"; break;
      case '\n': output << "\\n"; break;
      case '\r': output << "\\r"; break;
      case '\t': output << "\\t"; break;
      default:
        if (character < 0x20) {
          output << "\\u" << std::hex << std::setw(4) << std::setfill('0')
                 << static_cast<int>(character);
        } else {
          output << character;
        }
    }
  }
  return output.str();
}

std::string hresultMessage(HRESULT result) {
  _com_error error(result);
  const wchar_t* message = error.ErrorMessage();
  return message ? utf8(message) : "Windows 오류";
}

void checkStage(HRESULT result, const wchar_t* stage) {
  if (FAILED(result)) {
    throw hresult_error(result, hstring(stage));
  }
}

void writeTextAtomic(const fs::path& path, const std::string& value) {
  fs::create_directories(path.parent_path());
  const fs::path temporary = path.wstring()
    + L"." + std::to_wstring(GetCurrentProcessId())
    + L"." + std::to_wstring(epochMilliseconds()) + L".tmp";
  {
    std::ofstream stream(temporary, std::ios::binary | std::ios::trunc);
    if (!stream) throw std::runtime_error("임시 상태 파일을 열지 못했습니다");
    stream.write(value.data(), static_cast<std::streamsize>(value.size()));
    stream.flush();
    if (!stream) throw std::runtime_error("임시 상태 파일을 기록하지 못했습니다");
  }
  for (int attempt = 0; attempt < 5; ++attempt) {
    if (MoveFileExW(
      temporary.c_str(),
      path.c_str(),
      MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH
    )) return;
    std::this_thread::sleep_for(std::chrono::milliseconds(20 * (attempt + 1)));
  }
  DeleteFileW(temporary.c_str());
  throw std::runtime_error("상태 파일을 교체하지 못했습니다");
}

std::string readText(const fs::path& path) {
  std::ifstream stream(path, std::ios::binary);
  if (!stream) return {};
  return {
    std::istreambuf_iterator<char>(stream),
    std::istreambuf_iterator<char>()
  };
}

std::map<std::wstring, std::wstring> parseArguments(int count, wchar_t** values) {
  std::map<std::wstring, std::wstring> result;
  for (int index = 1; index < count; ++index) {
    std::wstring argument = values[index];
    if (argument.rfind(L"--", 0) != 0) continue;
    const auto separator = argument.find(L'=');
    if (separator == std::wstring::npos) {
      result[argument.substr(2)] = L"true";
    } else {
      result[argument.substr(2, separator - 2)] = argument.substr(separator + 1);
    }
  }
  return result;
}

int integerArgument(
  const std::map<std::wstring, std::wstring>& arguments,
  const std::wstring& name,
  int fallback,
  int minimum,
  int maximum
) {
  const auto iterator = arguments.find(name);
  if (iterator == arguments.end()) return fallback;
  try {
    const int value = std::stoi(iterator->second);
    return std::clamp(value, minimum, maximum);
  } catch (...) {
    return fallback;
  }
}

Options optionsFromArguments(int count, wchar_t** values) {
  const auto arguments = parseArguments(count, values);
  Options options;
  if (arguments.count(L"status-path")) options.statusPath = arguments.at(L"status-path");
  if (arguments.count(L"control-path")) options.controlPath = arguments.at(L"control-path");
  if (arguments.count(L"storage-path")) options.storagePath = arguments.at(L"storage-path");
  if (arguments.count(L"game-path")) options.gamePath = arguments.at(L"game-path");
  if (arguments.count(L"codec") && lower(arguments.at(L"codec")) == L"hevc") {
    options.codec = L"hevc";
  }
  options.fps = integerArgument(arguments, L"fps", 60, 15, 120);
  options.bitrateMbps = integerArgument(arguments, L"bitrate-mbps", 12, 2, 100);
  options.maxHeight = integerArgument(arguments, L"max-height", 1080, 0, 4320);
  options.chunkSeconds = integerArgument(arguments, L"chunk-seconds", 10, 2, 30);
  options.capacityGb = integerArgument(arguments, L"capacity-gb", 50, 1, 4096);
  options.maxDurationSeconds = integerArgument(
    arguments,
    L"max-duration-seconds",
    3600,
    60,
    604800
  );
  options.parentPid = static_cast<DWORD>(
    integerArgument(arguments, L"parent-pid", 0, 0, INT_MAX)
  );
  if (options.statusPath.empty() || options.controlPath.empty() || options.storagePath.empty()) {
    throw std::runtime_error("필수 실행 경로가 없습니다");
  }
  return options;
}

bool processRunning(DWORD pid) {
  if (pid == 0) return true;
  HANDLE process = OpenProcess(SYNCHRONIZE, FALSE, pid);
  if (!process) return false;
  const DWORD result = WaitForSingleObject(process, 0);
  CloseHandle(process);
  return result == WAIT_TIMEOUT;
}

std::wstring processImagePath(DWORD pid) {
  HANDLE process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
  if (!process) return {};
  std::wstring path(32768, L'\0');
  DWORD size = static_cast<DWORD>(path.size());
  if (!QueryFullProcessImageNameW(process, 0, path.data(), &size)) {
    CloseHandle(process);
    return {};
  }
  CloseHandle(process);
  path.resize(size);
  return path;
}

bool isMabinogiClientPath(const fs::path& executable, const fs::path& preferredPath) {
  if (lower(executable.filename().wstring()) != L"client.exe") return false;
  if (!preferredPath.empty() && lower(executable.wstring()) == lower(preferredPath.wstring())) {
    return true;
  }
  const auto parentName = lower(executable.parent_path().filename().wstring());
  if (parentName == L"mabinogi" || parentName == L"마비노기" || parentName == L"nexon") {
    return true;
  }
  std::error_code error;
  return fs::exists(executable.parent_path() / L"Mabinogi.exe", error);
}

struct WindowCandidate {
  fs::path preferredPath;
  HWND window = nullptr;
  long long area = 0;
};

BOOL CALLBACK enumerateGameWindows(HWND window, LPARAM parameter) {
  auto* candidate = reinterpret_cast<WindowCandidate*>(parameter);
  if (!IsWindowVisible(window) || IsIconic(window)) return TRUE;
  RECT client{};
  if (!GetClientRect(window, &client)) return TRUE;
  const long long width = client.right - client.left;
  const long long height = client.bottom - client.top;
  const long long area = width * height;
  if (width < 320 || height < 200 || area <= candidate->area) return TRUE;
  DWORD pid = 0;
  GetWindowThreadProcessId(window, &pid);
  const auto imagePath = processImagePath(pid);
  if (imagePath.empty() || !isMabinogiClientPath(imagePath, candidate->preferredPath)) return TRUE;
  candidate->window = window;
  candidate->area = area;
  return TRUE;
}

HWND findGameWindow(const fs::path& preferredPath) {
  WindowCandidate candidate{ preferredPath };
  EnumWindows(enumerateGameWindows, reinterpret_cast<LPARAM>(&candidate));
  return candidate.window;
}

void requireHardwareVideoEncoder(const std::wstring& codec) {
  MFT_REGISTER_TYPE_INFO inputType{
    MFMediaType_Video,
    MFVideoFormat_NV12,
  };
  MFT_REGISTER_TYPE_INFO outputType{
    MFMediaType_Video,
    codec == L"hevc" ? MFVideoFormat_HEVC : MFVideoFormat_H264,
  };
  IMFActivate** activations = nullptr;
  UINT32 activationCount = 0;
  const auto result = MFTEnumEx(
    MFT_CATEGORY_VIDEO_ENCODER,
    MFT_ENUM_FLAG_HARDWARE | MFT_ENUM_FLAG_SORTANDFILTER,
    &inputType,
    &outputType,
    &activations,
    &activationCount
  );
  if (activations) {
    for (UINT32 index = 0; index < activationCount; ++index) {
      if (activations[index]) activations[index]->Release();
    }
    CoTaskMemFree(activations);
  }
  checkStage(result, L"하드웨어 영상 인코더 검색");
  if (activationCount == 0) {
    throw std::runtime_error(
      codec == L"hevc"
        ? "HEVC 하드웨어 인코더를 사용할 수 없습니다"
        : "H.264 하드웨어 인코더를 사용할 수 없습니다"
    );
  }
}

IDirect3DDevice createWinrtDevice(
  ComPtr<ID3D11Device>& device,
  ComPtr<ID3D11DeviceContext>& context
) {
  UINT flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT;
  D3D_FEATURE_LEVEL level{};
  check_hresult(D3D11CreateDevice(
    nullptr,
    D3D_DRIVER_TYPE_HARDWARE,
    nullptr,
    flags,
    nullptr,
    0,
    D3D11_SDK_VERSION,
    &device,
    &level,
    &context
  ));
  ComPtr<ID3D10Multithread> multithread;
  check_hresult(context.As(&multithread));
  multithread->SetMultithreadProtected(TRUE);
  ComPtr<IDXGIDevice> dxgiDevice;
  check_hresult(device.As(&dxgiDevice));
  checkStage(dxgiDevice->SetGPUThreadPriority(-2), L"GPU 스레드 우선순위 제한");
  com_ptr<::IInspectable> inspectable;
  check_hresult(CreateDirect3D11DeviceFromDXGIDevice(dxgiDevice.Get(), inspectable.put()));
  return inspectable.as<IDirect3DDevice>();
}

GraphicsCaptureItem createCaptureItem(HWND window) {
  auto interop = get_activation_factory<GraphicsCaptureItem, IGraphicsCaptureItemInterop>();
  GraphicsCaptureItem item{ nullptr };
  check_hresult(interop->CreateForWindow(
    window,
    guid_of<GraphicsCaptureItem>(),
    put_abi(item)
  ));
  return item;
}

std::vector<fs::path> completedChunks(const fs::path& directory) {
  std::vector<fs::path> chunks;
  std::error_code error;
  if (!fs::exists(directory, error)) return chunks;
  for (const auto& entry : fs::directory_iterator(directory, error)) {
    if (error) break;
    if (!entry.is_regular_file(error)) continue;
    const auto name = lower(entry.path().filename().wstring());
    if (
      name.rfind(L"chunk-", 0) == 0
      && name.find(L".partial.") == std::wstring::npos
      && entry.path().extension() == L".mp4"
    ) {
      chunks.push_back(entry.path());
    }
  }
  std::sort(chunks.begin(), chunks.end(), [](const fs::path& left, const fs::path& right) {
    std::error_code leftError;
    std::error_code rightError;
    const auto leftTime = fs::last_write_time(left, leftError);
    const auto rightTime = fs::last_write_time(right, rightError);
    if (leftError || rightError) return left.filename() < right.filename();
    return leftTime < rightTime;
  });
  return chunks;
}

void removeIncompleteChunks(const fs::path& directory) {
  std::error_code error;
  if (!fs::exists(directory, error)) return;
  for (const auto& entry : fs::directory_iterator(directory, error)) {
    if (error) break;
    if (!entry.is_regular_file(error)) continue;
    const auto name = lower(entry.path().filename().wstring());
    if (
      name.rfind(L"chunk-", 0) == 0
      && name.find(L".partial.") != std::wstring::npos
      && entry.path().extension() == L".mp4"
    ) {
      std::error_code removeError;
      fs::remove(entry.path(), removeError);
    }
  }
}

void clearRingDirectory(const fs::path& directory) {
  for (const auto& path : completedChunks(directory)) {
    std::error_code error;
    fs::remove(path, error);
  }
  removeIncompleteChunks(directory);
}

std::int64_t chunkStartedMilliseconds(const fs::path& path);
LONGLONG compressedMediaDuration(const fs::path& input);

double completedChunkDurationSeconds(const fs::path& path) {
  try {
    return std::max(
      0.0,
      static_cast<double>(compressedMediaDuration(path)) / 10000000.0
    );
  } catch (...) {
    return 0;
  }
}

class RingStorageIndex {
public:
  explicit RingStorageIndex(fs::path directory)
    : directory_(std::move(directory)) {
    refresh();
  }

  std::uint64_t bytesUsed() const {
    return bytesUsed_.load();
  }

  double durationSeconds() const {
    return durationSeconds_.load();
  }

  std::uint64_t publish(
    const fs::path& path,
    std::uint64_t capacityBytes,
    std::int64_t maxDurationMilliseconds
  ) {
    std::lock_guard lock(mutex_);
    std::error_code sizeError;
    const auto size = fs::file_size(path, sizeError);
    if (sizeError) {
      refreshLocked();
      return bytesUsed_.load();
    }
    const auto started = chunkStartedMilliseconds(path);
    const auto duration = completedChunkDurationSeconds(path);
    chunks_.push_back({ path, size, started, duration });
    std::uint64_t total = bytesUsed_.load() + size;
    double totalDuration = durationSeconds_.load() + duration;
    std::error_code spaceError;
    auto space = fs::space(directory_, spaceError);
    const auto reserve = Gigabyte;
    const auto newestStarted = chunks_.back().started;
    while (
      !chunks_.empty()
      && (
        total > capacityBytes
        || (!spaceError && space.available < reserve)
        || (
          chunks_.size() > 1
          && maxDurationMilliseconds > 0
          && (
            chunks_.front().started <= 0
            || newestStarted - chunks_.front().started
              >= maxDurationMilliseconds
          )
        )
      )
    ) {
      auto entry = std::move(chunks_.front());
      chunks_.pop_front();
      std::error_code removeError;
      if (fs::remove(entry.path, removeError)) {
        total = total >= entry.size ? total - entry.size : 0;
        totalDuration = std::max(0.0, totalDuration - entry.duration);
        if (!spaceError) space.available += entry.size;
      } else {
        refreshLocked();
        return bytesUsed_.load();
      }
    }
    bytesUsed_ = total;
    durationSeconds_ = totalDuration;
    return total;
  }

  std::uint64_t clear() {
    std::lock_guard lock(mutex_);
    clearRingDirectory(directory_);
    refreshLocked();
    return bytesUsed_.load();
  }

private:
  struct Entry {
    fs::path path;
    std::uint64_t size = 0;
    std::int64_t started = 0;
    double duration = 0;
  };

  void refresh() {
    std::lock_guard lock(mutex_);
    refreshLocked();
  }

  void refreshLocked() {
    chunks_.clear();
    std::uint64_t total = 0;
    double totalDuration = 0;
    for (const auto& path : completedChunks(directory_)) {
      std::error_code error;
      const auto size = fs::file_size(path, error);
      if (error) continue;
      const auto duration = completedChunkDurationSeconds(path);
      chunks_.push_back({
        path,
        size,
        chunkStartedMilliseconds(path),
        duration
      });
      total += size;
      totalDuration += duration;
    }
    bytesUsed_ = total;
    durationSeconds_ = totalDuration;
  }

  fs::path directory_;
  mutable std::mutex mutex_;
  std::deque<Entry> chunks_;
  std::atomic_uint64_t bytesUsed_ = 0;
  std::atomic<double> durationSeconds_ = 0;
};

class Nv12Converter {
public:
  Nv12Converter(ID3D11Device* device, int width, int height, int fps, int maxHeight)
    : width_(0),
      height_(0) {
    const double scale = maxHeight > 0
      ? std::min(1.0, static_cast<double>(maxHeight) / static_cast<double>(height))
      : 1.0;
    width_ = std::max<UINT>(2, static_cast<UINT>(width * scale) & ~1u);
    height_ = std::max<UINT>(2, static_cast<UINT>(height * scale) & ~1u);
    device_ = device;
    check_hresult(device->QueryInterface(IID_PPV_ARGS(&videoDevice_)));
    ComPtr<ID3D11DeviceContext> context;
    device->GetImmediateContext(&context);
    check_hresult(context.As(&videoContext_));

    D3D11_VIDEO_PROCESSOR_CONTENT_DESC content{};
    content.InputFrameFormat = D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE;
    content.InputFrameRate = { static_cast<UINT>(fps), 1 };
    content.InputWidth = static_cast<UINT>(width);
    content.InputHeight = static_cast<UINT>(height);
    content.OutputFrameRate = { static_cast<UINT>(fps), 1 };
    content.OutputWidth = width_;
    content.OutputHeight = height_;
    content.Usage = D3D11_VIDEO_USAGE_PLAYBACK_NORMAL;
    outputDescription_.Width = width_;
    outputDescription_.Height = height_;
    outputDescription_.MipLevels = 1;
    outputDescription_.ArraySize = 1;
    outputDescription_.Format = DXGI_FORMAT_NV12;
    outputDescription_.SampleDesc = { 1, 0 };
    outputDescription_.Usage = D3D11_USAGE_DEFAULT;
    outputDescription_.BindFlags =
      D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE;
    check_hresult(videoDevice_->CreateVideoProcessorEnumerator(&content, &enumerator_));
    check_hresult(videoDevice_->CreateVideoProcessor(enumerator_.Get(), 0, &processor_));
    videoContext_->VideoProcessorSetStreamFrameFormat(
      processor_.Get(),
      0,
      D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE
    );
  }

  ComPtr<ID3D11Texture2D> convert(ID3D11Texture2D* source) {
    ComPtr<ID3D11Texture2D> output;
    check_hresult(device_->CreateTexture2D(&outputDescription_, nullptr, &output));

    const auto inputViewIterator = inputViews_.find(source);
    ComPtr<ID3D11VideoProcessorInputView> inputView;
    if (inputViewIterator != inputViews_.end()) {
      inputView = inputViewIterator->second;
    } else {
      D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC inputDescription{};
      inputDescription.FourCC = 0;
      inputDescription.ViewDimension = D3D11_VPIV_DIMENSION_TEXTURE2D;
      inputDescription.Texture2D.MipSlice = 0;
      inputDescription.Texture2D.ArraySlice = 0;
      check_hresult(videoDevice_->CreateVideoProcessorInputView(
        source,
        enumerator_.Get(),
        &inputDescription,
        &inputView
      ));
      inputViews_.emplace(source, inputView);
    }

    D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC outputViewDescription{};
    outputViewDescription.ViewDimension = D3D11_VPOV_DIMENSION_TEXTURE2D;
    outputViewDescription.Texture2D.MipSlice = 0;
    ComPtr<ID3D11VideoProcessorOutputView> outputView;
    check_hresult(videoDevice_->CreateVideoProcessorOutputView(
      output.Get(),
      enumerator_.Get(),
      &outputViewDescription,
      &outputView
    ));

    D3D11_VIDEO_PROCESSOR_STREAM stream{};
    stream.Enable = TRUE;
    stream.OutputIndex = 0;
    stream.InputFrameOrField = 0;
    stream.PastFrames = 0;
    stream.FutureFrames = 0;
    stream.pInputSurface = inputView.Get();
    checkStage(videoContext_->VideoProcessorBlt(
      processor_.Get(),
      outputView.Get(),
      0,
      1,
      &stream
    ), L"NV12 GPU 변환");
    return output;
  }

  UINT width() const {
    return width_;
  }

  UINT height() const {
    return height_;
  }

private:
  UINT width_;
  UINT height_;
  ComPtr<ID3D11Device> device_;
  ComPtr<ID3D11VideoDevice> videoDevice_;
  ComPtr<ID3D11VideoContext> videoContext_;
  ComPtr<ID3D11VideoProcessorEnumerator> enumerator_;
  ComPtr<ID3D11VideoProcessor> processor_;
  D3D11_TEXTURE2D_DESC outputDescription_{};
  std::unordered_map<
    ID3D11Texture2D*,
    ComPtr<ID3D11VideoProcessorInputView>
  > inputViews_;
};

class Mp4Writer {
public:
  Mp4Writer(
    ID3D11Device* device,
    const fs::path& path,
    int width,
    int height,
    int fps,
    int bitrateMbps,
    int maxHeight,
    const std::wstring& codec
  ) : path_(path), fps_(fps), converter_(device, width, height, fps, maxHeight) {
    ComPtr<IMFDXGIDeviceManager> manager;
    UINT resetToken = 0;
    check_hresult(MFCreateDXGIDeviceManager(&resetToken, &manager));
    check_hresult(manager->ResetDevice(device, resetToken));

    ComPtr<IMFAttributes> attributes;
    check_hresult(MFCreateAttributes(&attributes, 4));
    check_hresult(attributes->SetUINT32(MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, TRUE));
    check_hresult(attributes->SetUINT32(MF_LOW_LATENCY, TRUE));
    check_hresult(attributes->SetUnknown(MF_SINK_WRITER_D3D_MANAGER, manager.Get()));

    check_hresult(MFCreateSinkWriterFromURL(path.c_str(), nullptr, attributes.Get(), &writer_));

    ComPtr<IMFMediaType> outputType;
    check_hresult(MFCreateMediaType(&outputType));
    check_hresult(outputType->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video));
    check_hresult(outputType->SetGUID(
      MF_MT_SUBTYPE,
      codec == L"hevc" ? MFVideoFormat_HEVC : MFVideoFormat_H264
    ));
    check_hresult(outputType->SetUINT32(
      MF_MT_AVG_BITRATE,
      static_cast<UINT32>(bitrateMbps * 1000000)
    ));
    check_hresult(outputType->SetUINT32(MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive));
    check_hresult(MFSetAttributeSize(
      outputType.Get(),
      MF_MT_FRAME_SIZE,
      converter_.width(),
      converter_.height()
    ));
    check_hresult(MFSetAttributeRatio(outputType.Get(), MF_MT_FRAME_RATE, fps, 1));
    check_hresult(MFSetAttributeRatio(outputType.Get(), MF_MT_PIXEL_ASPECT_RATIO, 1, 1));
    check_hresult(writer_->AddStream(outputType.Get(), &streamIndex_));

    ComPtr<IMFMediaType> audioOutputType;
    check_hresult(MFCreateMediaType(&audioOutputType));
    check_hresult(audioOutputType->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Audio));
    check_hresult(audioOutputType->SetGUID(MF_MT_SUBTYPE, MFAudioFormat_AAC));
    check_hresult(audioOutputType->SetUINT32(MF_MT_AUDIO_NUM_CHANNELS, AudioChannels));
    check_hresult(audioOutputType->SetUINT32(
      MF_MT_AUDIO_SAMPLES_PER_SECOND,
      AudioSampleRate
    ));
    check_hresult(audioOutputType->SetUINT32(
      MF_MT_AUDIO_AVG_BYTES_PER_SECOND,
      24000
    ));
    check_hresult(audioOutputType->SetUINT32(MF_MT_AUDIO_BLOCK_ALIGNMENT, 1));
    check_hresult(audioOutputType->SetUINT32(
      MF_MT_AUDIO_BITS_PER_SAMPLE,
      AudioBitsPerSample
    ));
    check_hresult(audioOutputType->SetUINT32(MF_MT_AAC_PAYLOAD_TYPE, 0));
    check_hresult(writer_->AddStream(audioOutputType.Get(), &audioStreamIndex_));

    ComPtr<IMFMediaType> inputType;
    check_hresult(MFCreateMediaType(&inputType));
    check_hresult(inputType->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video));
    check_hresult(inputType->SetGUID(MF_MT_SUBTYPE, MFVideoFormat_NV12));
    check_hresult(inputType->SetUINT32(MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive));
    check_hresult(inputType->SetUINT32(MF_MT_DEFAULT_STRIDE, converter_.width()));
    check_hresult(MFSetAttributeSize(
      inputType.Get(),
      MF_MT_FRAME_SIZE,
      converter_.width(),
      converter_.height()
    ));
    check_hresult(MFSetAttributeRatio(inputType.Get(), MF_MT_FRAME_RATE, fps, 1));
    check_hresult(MFSetAttributeRatio(inputType.Get(), MF_MT_PIXEL_ASPECT_RATIO, 1, 1));
    check_hresult(writer_->SetInputMediaType(streamIndex_, inputType.Get(), nullptr));

    ComPtr<IMFMediaType> audioInputType;
    check_hresult(MFCreateMediaType(&audioInputType));
    check_hresult(audioInputType->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Audio));
    check_hresult(audioInputType->SetGUID(MF_MT_SUBTYPE, MFAudioFormat_PCM));
    check_hresult(audioInputType->SetUINT32(MF_MT_AUDIO_NUM_CHANNELS, AudioChannels));
    check_hresult(audioInputType->SetUINT32(
      MF_MT_AUDIO_SAMPLES_PER_SECOND,
      AudioSampleRate
    ));
    check_hresult(audioInputType->SetUINT32(
      MF_MT_AUDIO_BLOCK_ALIGNMENT,
      AudioBlockAlignment
    ));
    check_hresult(audioInputType->SetUINT32(
      MF_MT_AUDIO_AVG_BYTES_PER_SECOND,
      AudioSampleRate * AudioBlockAlignment
    ));
    check_hresult(audioInputType->SetUINT32(
      MF_MT_AUDIO_BITS_PER_SAMPLE,
      AudioBitsPerSample
    ));
    check_hresult(writer_->SetInputMediaType(
      audioStreamIndex_,
      audioInputType.Get(),
      nullptr
    ));
    check_hresult(writer_->BeginWriting());
  }

  ~Mp4Writer() {
    finalize();
  }

  void write(ID3D11Texture2D* texture, LONGLONG timestamp) {
    auto converted = converter_.convert(texture);
    ComPtr<IMFMediaBuffer> buffer;
    checkStage(MFCreateDXGISurfaceBuffer(
      __uuidof(ID3D11Texture2D),
      converted.Get(),
      0,
      FALSE,
      &buffer
    ), L"인코더 GPU 표면 생성");
    checkStage(
      buffer->SetCurrentLength(converter_.width() * converter_.height() * 3 / 2),
      L"인코더 GPU 표면 크기 설정"
    );
    ComPtr<IMFSample> sample;
    check_hresult(MFCreateSample(&sample));
    check_hresult(sample->AddBuffer(buffer.Get()));
    check_hresult(sample->SetSampleTime(timestamp));
    check_hresult(sample->SetSampleDuration(10000000ll / fps_));
    checkStage(writer_->WriteSample(streamIndex_, sample.Get()), L"하드웨어 인코더 프레임 기록");
  }

  void writeAudio(
    const std::vector<std::uint8_t>& samples,
    UINT32 frameCount,
    LONGLONG timestamp
  ) {
    if (samples.empty() || frameCount == 0) return;
    ComPtr<IMFMediaBuffer> buffer;
    check_hresult(MFCreateMemoryBuffer(
      static_cast<DWORD>(samples.size()),
      &buffer
    ));
    BYTE* destination = nullptr;
    check_hresult(buffer->Lock(&destination, nullptr, nullptr));
    std::memcpy(destination, samples.data(), samples.size());
    check_hresult(buffer->Unlock());
    check_hresult(buffer->SetCurrentLength(static_cast<DWORD>(samples.size())));

    ComPtr<IMFSample> sample;
    check_hresult(MFCreateSample(&sample));
    check_hresult(sample->AddBuffer(buffer.Get()));
    check_hresult(sample->SetSampleTime(timestamp));
    check_hresult(sample->SetSampleDuration(
      static_cast<LONGLONG>(frameCount) * 10000000ll / AudioSampleRate
    ));
    checkStage(
      writer_->WriteSample(audioStreamIndex_, sample.Get()),
      L"게임 오디오 AAC 기록"
    );
  }

  void finalize() {
    if (!writer_) return;
    const HRESULT result = writer_->Finalize();
    writer_.Reset();
    if (FAILED(result)) {
      std::error_code error;
      fs::remove(path_, error);
      check_hresult(result);
    }
  }

  const fs::path& path() const {
    return path_;
  }

private:
  fs::path path_;
  int fps_;
  Nv12Converter converter_;
  DWORD streamIndex_ = 0;
  DWORD audioStreamIndex_ = 0;
  ComPtr<IMFSinkWriter> writer_;
};

ComPtr<IMFSourceReader> createCompressedVideoReader(const fs::path& path) {
  ComPtr<IMFAttributes> attributes;
  check_hresult(MFCreateAttributes(&attributes, 1));
  check_hresult(attributes->SetUINT32(MF_READWRITE_DISABLE_CONVERTERS, TRUE));
  ComPtr<IMFSourceReader> reader;
  check_hresult(MFCreateSourceReaderFromURL(path.c_str(), attributes.Get(), &reader));
  check_hresult(reader->SetStreamSelection(
    static_cast<DWORD>(MF_SOURCE_READER_ALL_STREAMS),
    FALSE
  ));
  check_hresult(reader->SetStreamSelection(
    static_cast<DWORD>(MF_SOURCE_READER_FIRST_VIDEO_STREAM),
    TRUE
  ));
  return reader;
}

ComPtr<IMFSourceReader> createCompressedAudioReader(const fs::path& path) {
  ComPtr<IMFAttributes> attributes;
  check_hresult(MFCreateAttributes(&attributes, 1));
  check_hresult(attributes->SetUINT32(MF_READWRITE_DISABLE_CONVERTERS, TRUE));
  ComPtr<IMFSourceReader> reader;
  check_hresult(MFCreateSourceReaderFromURL(path.c_str(), attributes.Get(), &reader));
  ComPtr<IMFMediaType> nativeType;
  if (FAILED(reader->GetNativeMediaType(
    static_cast<DWORD>(MF_SOURCE_READER_FIRST_AUDIO_STREAM),
    0,
    &nativeType
  ))) return {};
  check_hresult(reader->SetStreamSelection(
    static_cast<DWORD>(MF_SOURCE_READER_ALL_STREAMS),
    FALSE
  ));
  check_hresult(reader->SetStreamSelection(
    static_cast<DWORD>(MF_SOURCE_READER_FIRST_AUDIO_STREAM),
    TRUE
  ));
  return reader;
}

ComPtr<IMFMediaType> createPcmAudioType() {
  ComPtr<IMFMediaType> type;
  check_hresult(MFCreateMediaType(&type));
  check_hresult(type->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Audio));
  check_hresult(type->SetGUID(MF_MT_SUBTYPE, MFAudioFormat_PCM));
  check_hresult(type->SetUINT32(MF_MT_AUDIO_NUM_CHANNELS, AudioChannels));
  check_hresult(type->SetUINT32(MF_MT_AUDIO_SAMPLES_PER_SECOND, AudioSampleRate));
  check_hresult(type->SetUINT32(MF_MT_AUDIO_BLOCK_ALIGNMENT, AudioBlockAlignment));
  check_hresult(type->SetUINT32(
    MF_MT_AUDIO_AVG_BYTES_PER_SECOND,
    AudioSampleRate * AudioBlockAlignment
  ));
  check_hresult(type->SetUINT32(MF_MT_AUDIO_BITS_PER_SAMPLE, AudioBitsPerSample));
  return type;
}

ComPtr<IMFMediaType> createAacAudioType() {
  ComPtr<IMFMediaType> type;
  check_hresult(MFCreateMediaType(&type));
  check_hresult(type->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Audio));
  check_hresult(type->SetGUID(MF_MT_SUBTYPE, MFAudioFormat_AAC));
  check_hresult(type->SetUINT32(MF_MT_AUDIO_NUM_CHANNELS, AudioChannels));
  check_hresult(type->SetUINT32(MF_MT_AUDIO_SAMPLES_PER_SECOND, AudioSampleRate));
  check_hresult(type->SetUINT32(MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 24000));
  check_hresult(type->SetUINT32(MF_MT_AUDIO_BLOCK_ALIGNMENT, 1));
  check_hresult(type->SetUINT32(MF_MT_AUDIO_BITS_PER_SAMPLE, AudioBitsPerSample));
  check_hresult(type->SetUINT32(MF_MT_AAC_PAYLOAD_TYPE, 0));
  return type;
}

ComPtr<IMFSourceReader> createPcmAudioReader(const fs::path& path) {
  ComPtr<IMFSourceReader> reader;
  check_hresult(MFCreateSourceReaderFromURL(path.c_str(), nullptr, &reader));
  ComPtr<IMFMediaType> nativeType;
  if (FAILED(reader->GetNativeMediaType(
    static_cast<DWORD>(MF_SOURCE_READER_FIRST_AUDIO_STREAM),
    0,
    &nativeType
  ))) return {};
  check_hresult(reader->SetStreamSelection(
    static_cast<DWORD>(MF_SOURCE_READER_ALL_STREAMS),
    FALSE
  ));
  check_hresult(reader->SetStreamSelection(
    static_cast<DWORD>(MF_SOURCE_READER_FIRST_AUDIO_STREAM),
    TRUE
  ));
  const auto pcmType = createPcmAudioType();
  check_hresult(reader->SetCurrentMediaType(
    static_cast<DWORD>(MF_SOURCE_READER_FIRST_AUDIO_STREAM),
    nullptr,
    pcmType.Get()
  ));
  return reader;
}

struct MediaSignature {
  GUID subtype{};
  UINT32 width = 0;
  UINT32 height = 0;
  UINT32 frameRateNumerator = 0;
  UINT32 frameRateDenominator = 0;
  UINT32 averageBitrate = 12000000;
  std::vector<std::uint8_t> sequenceHeader;
  bool hasAudio = false;
  GUID audioSubtype{};
  UINT32 audioSampleRate = 0;
  UINT32 audioChannels = 0;
  std::vector<std::uint8_t> audioUserData;
};

MediaSignature mediaSignature(IMFMediaType* mediaType) {
  MediaSignature signature;
  check_hresult(mediaType->GetGUID(MF_MT_SUBTYPE, &signature.subtype));
  check_hresult(MFGetAttributeSize(
    mediaType,
    MF_MT_FRAME_SIZE,
    &signature.width,
    &signature.height
  ));
  check_hresult(MFGetAttributeRatio(
    mediaType,
    MF_MT_FRAME_RATE,
    &signature.frameRateNumerator,
    &signature.frameRateDenominator
  ));
  mediaType->GetUINT32(MF_MT_AVG_BITRATE, &signature.averageBitrate);
  UINT32 headerSize = 0;
  if (
    SUCCEEDED(mediaType->GetBlobSize(MF_MT_MPEG_SEQUENCE_HEADER, &headerSize))
    && headerSize > 0
  ) {
    signature.sequenceHeader.resize(headerSize);
    check_hresult(mediaType->GetBlob(
      MF_MT_MPEG_SEQUENCE_HEADER,
      signature.sequenceHeader.data(),
      headerSize,
      nullptr
    ));
  }
  return signature;
}

MediaSignature mediaSignature(const fs::path& path) {
  auto reader = createCompressedVideoReader(path);
  ComPtr<IMFMediaType> mediaType;
  check_hresult(reader->GetCurrentMediaType(
    static_cast<DWORD>(MF_SOURCE_READER_FIRST_VIDEO_STREAM),
    &mediaType
  ));
  auto signature = mediaSignature(mediaType.Get());
  if (auto audioReader = createCompressedAudioReader(path)) {
    ComPtr<IMFMediaType> audioType;
    check_hresult(audioReader->GetCurrentMediaType(
      static_cast<DWORD>(MF_SOURCE_READER_FIRST_AUDIO_STREAM),
      &audioType
    ));
    signature.hasAudio = true;
    check_hresult(audioType->GetGUID(MF_MT_SUBTYPE, &signature.audioSubtype));
    check_hresult(audioType->GetUINT32(
      MF_MT_AUDIO_SAMPLES_PER_SECOND,
      &signature.audioSampleRate
    ));
    check_hresult(audioType->GetUINT32(
      MF_MT_AUDIO_NUM_CHANNELS,
      &signature.audioChannels
    ));
    UINT32 userDataSize = 0;
    if (
      SUCCEEDED(audioType->GetBlobSize(MF_MT_USER_DATA, &userDataSize))
      && userDataSize > 0
    ) {
      signature.audioUserData.resize(userDataSize);
      check_hresult(audioType->GetBlob(
        MF_MT_USER_DATA,
        signature.audioUserData.data(),
        userDataSize,
        nullptr
      ));
    }
  }
  return signature;
}

bool sameMediaSignature(const MediaSignature& left, const MediaSignature& right) {
  return left.subtype == right.subtype
    && left.width == right.width
    && left.height == right.height
    && left.sequenceHeader == right.sequenceHeader
    && left.hasAudio == right.hasAudio
    && (!left.hasAudio || (
      left.audioSubtype == right.audioSubtype
      && left.audioSampleRate == right.audioSampleRate
      && left.audioChannels == right.audioChannels
      && left.audioUserData == right.audioUserData
    ));
}

bool sameMediaFormat(const MediaSignature& left, const MediaSignature& right) {
  return left.subtype == right.subtype
    && left.width == right.width
    && left.height == right.height
    && left.hasAudio == right.hasAudio
    && (!left.hasAudio || (
      left.audioSubtype == right.audioSubtype
      && left.audioSampleRate == right.audioSampleRate
      && left.audioChannels == right.audioChannels
    ));
}

LONGLONG fallbackSampleDuration(const MediaSignature& signature) {
  if (signature.frameRateNumerator == 0 || signature.frameRateDenominator == 0) {
    return 10000000ll / 60;
  }
  return static_cast<LONGLONG>(
    10000000ull * signature.frameRateDenominator / signature.frameRateNumerator
  );
}

void checkReaderFlags(DWORD flags) {
  if (flags & MF_SOURCE_READERF_ERROR) {
    throw std::runtime_error("압축 영상 sample을 읽지 못했습니다");
  }
  if (flags & MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED) {
    throw std::runtime_error("편집 중 영상 형식이 변경되었습니다");
  }
}

LONGLONG compressedMediaDuration(const fs::path& input) {
  auto reader = createCompressedVideoReader(input);
  PROPVARIANT presentationDuration;
  PropVariantInit(&presentationDuration);
  const auto presentationResult = reader->GetPresentationAttribute(
    static_cast<DWORD>(MF_SOURCE_READER_MEDIASOURCE),
    MF_PD_DURATION,
    &presentationDuration
  );
  LONGLONG duration = 0;
  if (SUCCEEDED(presentationResult)) {
    if (presentationDuration.vt == VT_UI8) {
      duration = static_cast<LONGLONG>(presentationDuration.uhVal.QuadPart);
    } else if (presentationDuration.vt == VT_I8) {
      duration = presentationDuration.hVal.QuadPart;
    }
  }
  PropVariantClear(&presentationDuration);
  if (duration > 0) return duration;

  ComPtr<IMFMediaType> mediaType;
  check_hresult(reader->GetCurrentMediaType(
    static_cast<DWORD>(MF_SOURCE_READER_FIRST_VIDEO_STREAM),
    &mediaType
  ));
  const auto signature = mediaSignature(mediaType.Get());
  const LONGLONG defaultDuration = fallbackSampleDuration(signature);
  LONGLONG firstTime = -1;
  LONGLONG fileEnd = 0;
  while (true) {
    DWORD actualStream = 0;
    DWORD flags = 0;
    LONGLONG timestamp = 0;
    ComPtr<IMFSample> sample;
    check_hresult(reader->ReadSample(
      static_cast<DWORD>(MF_SOURCE_READER_FIRST_VIDEO_STREAM),
      0,
      &actualStream,
      &flags,
      &timestamp,
      &sample
    ));
    checkReaderFlags(flags);
    if (flags & MF_SOURCE_READERF_ENDOFSTREAM) break;
    if (!sample) continue;
    if (firstTime < 0) firstTime = timestamp;
    LONGLONG sampleDuration = 0;
    if (FAILED(sample->GetSampleDuration(&sampleDuration)) || sampleDuration <= 0) {
      sampleDuration = defaultDuration;
    }
    fileEnd = std::max(
      fileEnd,
      std::max<LONGLONG>(0, timestamp - firstTime) + sampleDuration
    );
  }
  return fileEnd;
}

/*
  각 Ring 청크는 독립 재생 가능한 MP4로 확정되므로 첫 영상 sample이
  디코딩 시작점이다. 편집 시작점은 청크 경계로 내리고 실제 표시는
  requestedStart 기준 timestamp를 유지해 전체 sample 사전 탐색을 피한다.
*/
LONGLONG findCleanRangeStart(
  const std::vector<fs::path>& inputs,
  LONGLONG requestedStart
) {
  if (requestedStart <= 0) return 0;
  LONGLONG inputOffset = 0;
  for (const auto& input : inputs) {
    const auto inputDuration = compressedMediaDuration(input);
    if (inputOffset + inputDuration > requestedStart) return inputOffset;
    inputOffset += inputDuration;
  }
  return std::max<LONGLONG>(0, inputOffset);
}

LONGLONG combinedMediaDuration(const std::vector<fs::path>& inputs) {
  LONGLONG duration = 0;
  for (const auto& input : inputs) {
    duration += compressedMediaDuration(input);
  }
  return duration;
}

ComPtr<IMFSample> retimePcmSample(
  IMFSample* sample,
  double playbackRate,
  LONGLONG& duration,
  LONGLONG maximumInputDuration = LLONG_MAX
) {
  ComPtr<IMFMediaBuffer> inputBuffer;
  check_hresult(sample->ConvertToContiguousBuffer(&inputBuffer));
  BYTE* inputData = nullptr;
  DWORD inputBytes = 0;
  check_hresult(inputBuffer->Lock(&inputData, nullptr, &inputBytes));
  const auto inputFrames = inputBytes / AudioBlockAlignment;
  if (inputFrames == 0) {
    check_hresult(inputBuffer->Unlock());
    return sample;
  }
  const auto maximumInputFrames = maximumInputDuration == LLONG_MAX
    ? static_cast<std::uint64_t>(inputFrames)
    : static_cast<std::uint64_t>(std::max<LONGLONG>(
      0,
      maximumInputDuration * AudioSampleRate / 10000000ll
    ));
  const auto usableInputFrames = std::min<std::uint64_t>(
    inputFrames,
    maximumInputFrames
  );
  if (
    usableInputFrames == inputFrames
    && std::abs(playbackRate - 1.0) < 0.001
  ) {
    check_hresult(inputBuffer->Unlock());
    return sample;
  }
  if (usableInputFrames == 0) {
    check_hresult(inputBuffer->Unlock());
    return {};
  }
  const auto outputFrames = std::max<std::uint64_t>(
    1,
    static_cast<std::uint64_t>(std::llround(usableInputFrames / playbackRate))
  );
  ComPtr<IMFMediaBuffer> outputBuffer;
  check_hresult(MFCreateMemoryBuffer(
    static_cast<DWORD>(outputFrames * AudioBlockAlignment),
    &outputBuffer
  ));
  BYTE* outputData = nullptr;
  check_hresult(outputBuffer->Lock(&outputData, nullptr, nullptr));
  for (std::uint64_t index = 0; index < outputFrames; ++index) {
    const auto sourceIndex = std::min<std::uint64_t>(
      usableInputFrames - 1,
      static_cast<std::uint64_t>(index * playbackRate)
    );
    std::memcpy(
      outputData + index * AudioBlockAlignment,
      inputData + sourceIndex * AudioBlockAlignment,
      AudioBlockAlignment
    );
  }
  check_hresult(outputBuffer->Unlock());
  check_hresult(inputBuffer->Unlock());
  check_hresult(outputBuffer->SetCurrentLength(
    static_cast<DWORD>(outputFrames * AudioBlockAlignment)
  ));
  ComPtr<IMFSample> outputSample;
  check_hresult(MFCreateSample(&outputSample));
  check_hresult(outputSample->AddBuffer(outputBuffer.Get()));
  duration = static_cast<LONGLONG>(
    outputFrames * 10000000ull / AudioSampleRate
  );
  return outputSample;
}

using RemuxProgress = std::function<void(std::size_t, std::size_t)>;

bool remuxChunks(
  const std::vector<fs::path>& inputs,
  const fs::path& output,
  LONGLONG requestedStart = 0,
  LONGLONG requestedDuration = LLONG_MAX,
  bool allowFormatChanges = false,
  double playbackRate = 1.0,
  RemuxProgress onProgress = {}
) {
  if (inputs.empty()) return false;
  const LONGLONG effectiveStart = findCleanRangeStart(inputs, requestedStart);
  const LONGLONG requestedEnd = requestedDuration == LLONG_MAX
    ? LLONG_MAX
    : requestedStart + requestedDuration;
  const LONGLONG audioRequestedStart = requestedDuration == LLONG_MAX
    ? 0
    : requestedStart;
  const LONGLONG audioRequestedEnd = requestedDuration == LLONG_MAX
    ? LLONG_MAX
    : audioRequestedStart + requestedDuration;
  ComPtr<IMFSinkWriter> sink;
  DWORD sinkStream = 0;
  std::optional<DWORD> sinkAudioStream;
  LONGLONG outputTime = 0;
  LONGLONG retimedAudioOutputTime = 0;
  std::optional<MediaSignature> expectedSignature;

  for (std::size_t fileIndex = 0; fileIndex < inputs.size(); ++fileIndex) {
    auto reader = createCompressedVideoReader(inputs[fileIndex]);

    ComPtr<IMFMediaType> mediaType;
    check_hresult(reader->GetCurrentMediaType(
      static_cast<DWORD>(MF_SOURCE_READER_FIRST_VIDEO_STREAM),
      &mediaType
    ));
    const auto signature = mediaSignature(inputs[fileIndex]);
    if (
      expectedSignature
      && !(
        allowFormatChanges
          ? sameMediaFormat(*expectedSignature, signature)
          : sameMediaSignature(*expectedSignature, signature)
      )
    ) {
      throw std::runtime_error("해상도 또는 인코딩 형식이 다른 청크는 결합할 수 없습니다");
    }
    expectedSignature = signature;
    const auto fileDuration = compressedMediaDuration(inputs[fileIndex]);
    if (outputTime + fileDuration <= effectiveStart) {
      outputTime += fileDuration;
      if (onProgress) onProgress(fileIndex + 1, inputs.size());
      continue;
    }
    if (outputTime >= requestedEnd) break;
    const LONGLONG defaultDuration = fallbackSampleDuration(signature);
    ComPtr<IMFSourceReader> audioReader;
    if (signature.hasAudio) audioReader = createPcmAudioReader(inputs[fileIndex]);
    ComPtr<IMFMediaType> audioInputType;
    if (audioReader) {
      check_hresult(audioReader->GetCurrentMediaType(
        static_cast<DWORD>(MF_SOURCE_READER_FIRST_AUDIO_STREAM),
        &audioInputType
      ));
    }
    if (!sink) {
      ComPtr<IMFAttributes> sinkAttributes;
      check_hresult(MFCreateAttributes(&sinkAttributes, 1));
      check_hresult(sinkAttributes->SetUINT32(
        MF_SINK_WRITER_DISABLE_THROTTLING,
        TRUE
      ));
      check_hresult(MFCreateSinkWriterFromURL(
        output.c_str(),
        nullptr,
        sinkAttributes.Get(),
        &sink
      ));
      check_hresult(sink->AddStream(mediaType.Get(), &sinkStream));
      check_hresult(sink->SetInputMediaType(sinkStream, mediaType.Get(), nullptr));
      if (audioInputType) {
        const auto audioOutputType = createAacAudioType();
        DWORD audioStream = 0;
        check_hresult(sink->AddStream(audioOutputType.Get(), &audioStream));
        check_hresult(sink->SetInputMediaType(
          audioStream,
          audioInputType.Get(),
          nullptr
        ));
        sinkAudioStream = audioStream;
      }
      check_hresult(sink->BeginWriting());
    }

    const LONGLONG fileStart = outputTime;
    LONGLONG firstTime = -1;
    LONGLONG fileEnd = outputTime;
    while (true) {
      DWORD actualStream = 0;
      DWORD flags = 0;
      LONGLONG timestamp = 0;
      ComPtr<IMFSample> sample;
      check_hresult(reader->ReadSample(
        static_cast<DWORD>(MF_SOURCE_READER_FIRST_VIDEO_STREAM),
        0,
        &actualStream,
        &flags,
        &timestamp,
        &sample
      ));
      checkReaderFlags(flags);
      if (flags & MF_SOURCE_READERF_ENDOFSTREAM) break;
      if (!sample) continue;
      if (firstTime < 0) firstTime = timestamp;
      LONGLONG duration = 0;
      if (FAILED(sample->GetSampleDuration(&duration)) || duration <= 0) {
        duration = defaultDuration;
      }
      const LONGLONG globalTime =
        outputTime + std::max<LONGLONG>(0, timestamp - firstTime);
      fileEnd = std::max(fileEnd, globalTime + duration);
      if (globalTime + duration <= effectiveStart) continue;
      if (globalTime >= requestedEnd) break;
      const auto outputSampleTime = static_cast<LONGLONG>(
        std::llround((globalTime - requestedStart) / playbackRate)
      );
      const auto outputSampleDuration = std::max<LONGLONG>(
        1,
        static_cast<LONGLONG>(std::llround(duration / playbackRate))
      );
      check_hresult(sample->SetSampleTime(outputSampleTime));
      check_hresult(sample->SetSampleDuration(outputSampleDuration));
      UINT64 decodeTimestamp = 0;
      if (SUCCEEDED(sample->GetUINT64(
        MFSampleExtension_DecodeTimestamp,
        &decodeTimestamp
      ))) {
        const auto signedDecodeTimestamp = static_cast<LONGLONG>(decodeTimestamp);
        const auto globalDecodeTimestamp =
          outputTime + signedDecodeTimestamp - firstTime;
        const auto outputDecodeTimestamp = static_cast<LONGLONG>(
          std::llround((globalDecodeTimestamp - requestedStart) / playbackRate)
        );
        check_hresult(sample->SetUINT64(
          MFSampleExtension_DecodeTimestamp,
          static_cast<UINT64>(outputDecodeTimestamp)
        ));
      }
      check_hresult(sink->WriteSample(sinkStream, sample.Get()));
    }
    outputTime = fileEnd;

    if (audioReader && sinkAudioStream) {
      const LONGLONG defaultAudioDuration =
        signature.audioSampleRate > 0
          ? 1024ll * 10000000ll / signature.audioSampleRate
          : 1024ll * 10000000ll / AudioSampleRate;
      LONGLONG fileAudioTime = 0;
      while (true) {
        DWORD actualStream = 0;
        DWORD flags = 0;
        LONGLONG timestamp = 0;
        ComPtr<IMFSample> sample;
        check_hresult(audioReader->ReadSample(
          static_cast<DWORD>(MF_SOURCE_READER_FIRST_AUDIO_STREAM),
          0,
          &actualStream,
          &flags,
          &timestamp,
          &sample
        ));
        checkReaderFlags(flags);
        if (flags & MF_SOURCE_READERF_ENDOFSTREAM) break;
        if (!sample) continue;
        LONGLONG duration = 0;
        if (FAILED(sample->GetSampleDuration(&duration)) || duration <= 0) {
          duration = defaultAudioDuration;
        }
        LONGLONG relativeTime = timestamp;
        if (
          relativeTime < 0
          || relativeTime + 2500000ll < fileAudioTime
          || relativeTime > fileAudioTime + 2500000ll
        ) {
          relativeTime = fileAudioTime;
        }
        const LONGLONG globalTime = fileStart + relativeTime;
        fileAudioTime = std::max(fileAudioTime, relativeTime + duration);
        if (globalTime + duration <= audioRequestedStart) continue;
        if (globalTime >= audioRequestedEnd) break;
        const auto availableInputDuration = std::min(
          fileEnd,
          audioRequestedEnd
        ) - globalTime;
        if (availableInputDuration <= 0) break;
        sample = retimePcmSample(
          sample.Get(),
          playbackRate,
          duration,
          availableInputDuration
        );
        if (!sample) break;
        const auto mappedOutputTime = std::max<LONGLONG>(
          0,
          static_cast<LONGLONG>(std::llround(
            (globalTime - audioRequestedStart) / playbackRate
          ))
        );
        const auto sampleTime = std::max(
          retimedAudioOutputTime,
          mappedOutputTime
        );
        check_hresult(sample->SetSampleTime(sampleTime));
        check_hresult(sample->SetSampleDuration(duration));
        sample->DeleteItem(MFSampleExtension_DecodeTimestamp);
        check_hresult(sink->WriteSample(*sinkAudioStream, sample.Get()));
        retimedAudioOutputTime = sampleTime + duration;
      }
    }
    if (onProgress) onProgress(fileIndex + 1, inputs.size());
  }
  if (!sink) return false;
  check_hresult(sink->Finalize());
  return true;
}

bool remuxChunksAtomically(
  const std::vector<fs::path>& inputs,
  const fs::path& output,
  LONGLONG requestedStart = 0,
  LONGLONG requestedDuration = LLONG_MAX,
  bool allowFormatChanges = false,
  double playbackRate = 1.0,
  RemuxProgress onProgress = {}
) {
  auto partial = output;
  partial += L".partial.mp4";
  std::error_code cleanupError;
  fs::remove(partial, cleanupError);
  try {
    if (!remuxChunks(
      inputs,
      partial,
      requestedStart,
      requestedDuration,
      allowFormatChanges,
      playbackRate,
      onProgress
    )) return false;
    fs::remove(output, cleanupError);
    fs::rename(partial, output);
    return true;
  } catch (...) {
    fs::remove(partial, cleanupError);
    throw;
  }
}

std::vector<fs::path> selectRecentChunks(const fs::path& directory, int seconds) {
  auto chunks = completedChunks(directory);
  if (chunks.empty()) return {};
  const auto threshold = fs::file_time_type::clock::now() - std::chrono::seconds(seconds + 4);
  auto first = std::find_if(chunks.begin(), chunks.end(), [&](const fs::path& path) {
    std::error_code error;
    return fs::last_write_time(path, error) >= threshold;
  });
  if (first == chunks.end()) {
    return { chunks.back() };
  }
  return { first, chunks.end() };
}

std::int64_t chunkStartedMilliseconds(const fs::path& path) {
  const auto name = path.stem().wstring();
  constexpr std::wstring_view prefix = L"chunk-";
  if (name.rfind(prefix, 0) != 0) return 0;
  try {
    return std::stoll(name.substr(prefix.size()));
  } catch (...) {
    return 0;
  }
}

std::vector<fs::path> selectAnchoredChunks(
  const fs::path& directory,
  std::int64_t anchorMilliseconds,
  int seconds
) {
  auto chunks = completedChunks(directory);
  const auto earliest = anchorMilliseconds - static_cast<std::int64_t>(seconds) * 1000;
  chunks.erase(
    std::remove_if(chunks.begin(), chunks.end(), [&](const fs::path& path) {
      const auto started = chunkStartedMilliseconds(path);
      return started <= 0
        || started < earliest - 30000
        || started > anchorMilliseconds;
    }),
    chunks.end()
  );
  return chunks;
}

struct TrackTimelineMetadata {
  std::string json;
  LONGLONG mediaDuration;
};

TrackTimelineMetadata trackTimelineMetadata(
  const std::vector<fs::path>& chunks,
  std::int64_t anchorMilliseconds,
  int seconds
) {
  struct Segment {
    double timelineStart;
    double mediaStart;
    double duration;
  };
  struct Chunk {
    std::wstring fileName;
    double timelineStart;
    double mediaStart;
    double duration;
  };

  const auto windowStart =
    anchorMilliseconds - static_cast<std::int64_t>(seconds) * 1000;
  constexpr double continuityToleranceSeconds = 1.5;
  std::vector<Segment> segments;
  std::vector<Chunk> visibleChunks;
  double mediaCursor = 0.0;
  LONGLONG mediaDuration = 0;
  double previousTimelineEnd = 0.0;
  double previousRawTimelineEnd = 0.0;

  for (const auto& chunk : chunks) {
    const auto started = chunkStartedMilliseconds(chunk);
    const auto durationTicks = compressedMediaDuration(chunk);
    const auto duration = static_cast<double>(durationTicks) / 10000000.0;
    if (started <= 0 || duration <= 0.0) continue;
    const auto rawTimelineStart =
      static_cast<double>(started - windowStart) / 1000.0;
    const auto rawTimelineEnd = rawTimelineStart + duration;
    if (rawTimelineEnd <= 0.0 || rawTimelineStart >= static_cast<double>(seconds)) {
      mediaDuration += durationTicks;
      mediaCursor += duration;
      continue;
    }
    const auto mediaOffset = std::max(0.0, -rawTimelineStart);
    double timelineStart = std::clamp(
      rawTimelineStart,
      0.0,
      static_cast<double>(seconds)
    );
    bool joinsPreviousSegment = false;
    if (!segments.empty()) {
      const auto rawGap = rawTimelineStart - previousRawTimelineEnd;
      joinsPreviousSegment = rawGap <= continuityToleranceSeconds;
      timelineStart = joinsPreviousSegment
        ? previousTimelineEnd
        : previousTimelineEnd + rawGap;
    }
    const auto visibleDuration = std::min(
      duration - mediaOffset,
      std::max(0.0, static_cast<double>(seconds) - timelineStart)
    );
    if (visibleDuration > 0.0) {
      visibleChunks.push_back({
        chunk.filename().wstring(),
        timelineStart,
        mediaOffset,
        visibleDuration
      });
      if (joinsPreviousSegment) {
        segments.back().duration += visibleDuration;
      } else {
        segments.push_back({
          timelineStart,
          mediaCursor + mediaOffset,
          visibleDuration
        });
      }
      previousTimelineEnd = timelineStart + visibleDuration;
    }
    previousRawTimelineEnd = rawTimelineStart + duration;
    mediaDuration += durationTicks;
    mediaCursor += duration;
  }

  std::ostringstream output;
  output.imbue(std::locale::classic());
  output << std::fixed << std::setprecision(3);
  output << "{\"timelineDurationSeconds\":" << seconds
    << ",\"mediaDurationSeconds\":" << mediaCursor
    << ",\"segments\":[";
  for (std::size_t index = 0; index < segments.size(); ++index) {
    if (index > 0) output << ',';
    const auto& segment = segments[index];
    output << "{\"timelineStartSeconds\":" << segment.timelineStart
      << ",\"mediaStartSeconds\":" << segment.mediaStart
      << ",\"durationSeconds\":" << segment.duration << '}';
  }
  output << "],\"chunks\":[";
  for (std::size_t index = 0; index < visibleChunks.size(); ++index) {
    if (index > 0) output << ',';
    const auto& chunk = visibleChunks[index];
    output << "{\"fileName\":\"" << jsonEscape(chunk.fileName) << '"'
      << ",\"timelineStartSeconds\":" << chunk.timelineStart
      << ",\"mediaStartSeconds\":" << chunk.mediaStart
      << ",\"durationSeconds\":" << chunk.duration << '}';
  }
  output << "],\"gaps\":[";
  double gapStart = 0.0;
  bool firstGap = true;
  for (const auto& segment : segments) {
    if (segment.timelineStart > gapStart + 0.01) {
      if (!firstGap) output << ',';
      output << "{\"startSeconds\":" << gapStart
        << ",\"durationSeconds\":" << segment.timelineStart - gapStart << '}';
      firstGap = false;
    }
    gapStart = std::max(gapStart, segment.timelineStart + segment.duration);
  }
  if (gapStart < static_cast<double>(seconds) - 0.01) {
    if (!firstGap) output << ',';
    output << "{\"startSeconds\":" << gapStart
      << ",\"durationSeconds\":" << static_cast<double>(seconds) - gapStart << '}';
  }
  output << "]}";
  return { output.str(), mediaDuration };
}

std::vector<fs::path> compatibleChunkSuffix(
  const std::vector<fs::path>& chunks
) {
  if (chunks.empty()) return {};
  std::optional<MediaSignature> expected;
  std::size_t firstCompatible = chunks.size();
  std::size_t endCompatible = chunks.size();
  while (firstCompatible > 0) {
    const auto candidateIndex = firstCompatible - 1;
    try {
      const auto candidate = mediaSignature(chunks[candidateIndex]);
      if (!expected) {
        expected = candidate;
        firstCompatible = candidateIndex;
        endCompatible = candidateIndex + 1;
        continue;
      }
      if (!sameMediaSignature(*expected, candidate)) break;
      firstCompatible = candidateIndex;
    } catch (...) {
      if (expected) break;
      firstCompatible = candidateIndex;
      endCompatible = candidateIndex;
    }
  }
  if (!expected) return {};
  return {
    chunks.begin() + static_cast<std::ptrdiff_t>(firstCompatible),
    chunks.begin() + static_cast<std::ptrdiff_t>(endCompatible),
  };
}

void createClip(
  const fs::path& ringDirectory,
  int seconds,
  SharedStatus& status
) {
  SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_LOWEST);
  {
    std::lock_guard lock(status.mutex);
    if (status.clipInProgress) return;
    status.clipInProgress = true;
    status.error.clear();
  }
  init_apartment(apartment_type::multi_threaded);
  try {
      const auto sourceFiles = compatibleChunkSuffix(
        selectRecentChunks(ringDirectory, seconds)
      );
      if (sourceFiles.empty()) throw std::runtime_error("저장할 녹화 청크가 없습니다");
      const auto identifier = std::to_wstring(epochMilliseconds());
      const auto clipsDirectory = ringDirectory.parent_path() / L"Clips";
      const auto stagingDirectory = clipsDirectory / (L".staging-" + identifier);
      fs::create_directories(stagingDirectory);
      std::vector<fs::path> protectedFiles;
      for (std::size_t index = 0; index < sourceFiles.size(); ++index) {
        const auto destination = stagingDirectory
          / (L"part-" + std::to_wstring(index) + L".mp4");
        if (!CreateHardLinkW(destination.c_str(), sourceFiles[index].c_str(), nullptr)) {
          fs::copy_file(sourceFiles[index], destination, fs::copy_options::overwrite_existing);
        }
        protectedFiles.push_back(destination);
      }
      const auto output = clipsDirectory / (L"마비노기-클립-" + identifier + L".mp4");
      const auto totalDuration = combinedMediaDuration(protectedFiles);
      const auto requestedDuration = std::min<LONGLONG>(
        totalDuration,
        static_cast<LONGLONG>(seconds) * 10000000ll
      );
      const auto requestedStart = std::max<LONGLONG>(
        0,
        totalDuration - requestedDuration
      );
      if (!remuxChunksAtomically(
        protectedFiles,
        output,
        requestedStart,
        requestedDuration
      )) {
        throw std::runtime_error("클립 파일을 결합하지 못했습니다");
      }
      std::error_code cleanupError;
      fs::remove_all(stagingDirectory, cleanupError);
      std::lock_guard lock(status.mutex);
      status.latestClip = output.wstring();
  } catch (const hresult_error& error) {
    std::lock_guard lock(status.mutex);
    status.error = std::wstring(L"클립 저장 실패: ") + error.message().c_str();
  } catch (const std::exception& error) {
    std::lock_guard lock(status.mutex);
    const auto message = error.what();
    const int length = MultiByteToWideChar(CP_UTF8, 0, message, -1, nullptr, 0);
    std::wstring wide(static_cast<std::size_t>(std::max(0, length)), L'\0');
    if (length > 1) {
      MultiByteToWideChar(CP_UTF8, 0, message, -1, wide.data(), length);
      wide.resize(static_cast<std::size_t>(length - 1));
    }
    status.error = std::wstring(L"클립 저장 실패: ") + wide;
  }
  std::lock_guard lock(status.mutex);
  status.clipInProgress = false;
}

class EncoderWorker {
public:
  EncoderWorker(
    ComPtr<ID3D11Device> device,
    fs::path ringDirectory,
    RingStorageIndex& ringStorage,
    const Options& options,
    SharedStatus& status
  ) : device_(std::move(device)),
      ringDirectory_(std::move(ringDirectory)),
      ringStorage_(ringStorage),
      options_(options),
      status_(status),
      thread_([this] { run(); }) {
  }

  ~EncoderWorker() {
    stop();
  }

  bool enqueue(FramePacket&& packet) {
    std::lock_guard lock(mutex_);
    if (stopping_ || failed_ || queue_.size() >= MaximumQueuedFrames) return false;
    queue_.push(std::move(packet));
    condition_.notify_one();
    return true;
  }

  bool enqueueAudio(AudioPacket&& packet) {
    std::lock_guard lock(mutex_);
    if (
      stopping_
      || failed_
      || packet.frameCount == 0
      || packet.frameCount > MaximumQueuedAudioFrames
      || queuedAudioFrames_ + packet.frameCount > MaximumQueuedAudioFrames
    ) return false;
    queuedAudioFrames_ += packet.frameCount;
    audioQueue_.push(std::move(packet));
    condition_.notify_one();
    return true;
  }

  void flush() {
    std::lock_guard lock(mutex_);
    flushRequested_ = true;
    condition_.notify_one();
  }

  void requestFlush(std::uint64_t requestId) {
    std::lock_guard lock(mutex_);
    flushRequestId_ = requestId;
    flushRequested_ = true;
    condition_.notify_one();
  }

  void requestClip(int seconds) {
    std::lock_guard lock(mutex_);
    clipSeconds_ = seconds;
    flushRequested_ = true;
    condition_.notify_one();
  }

  void requestClear(std::uint64_t requestId) {
    std::lock_guard lock(mutex_);
    clearRequestId_ = requestId;
    flushRequested_ = true;
    condition_.notify_one();
  }

  bool failed() const {
    return failed_;
  }

  double currentChunkDurationSeconds() const {
    return currentChunkDurationSeconds_.load();
  }

  void stop() {
    {
      std::lock_guard lock(mutex_);
      if (stopping_) return;
      stopping_ = true;
      condition_.notify_one();
    }
    if (thread_.joinable()) thread_.join();
    if (clipThread_.joinable()) clipThread_.join();
  }

private:
  void discardQueuedVideoFrames() {
    std::vector<FramePacket> discarded;
    {
      std::lock_guard lock(mutex_);
      while (!queue_.empty()) {
        discarded.push_back(std::move(queue_.front()));
        queue_.pop();
      }
    }
    for (auto& packet : discarded) {
      if (packet.texturePool) {
        packet.texturePool->release(std::move(packet.texture));
      }
    }
    if (!discarded.empty()) {
      status_.droppedFrames.fetch_add(discarded.size());
    }
  }

  void joinWriterPublisher() {
    if (writerPublisherThread_.joinable()) writerPublisherThread_.join();
  }

  void closeWriter(bool waitForPublish = false) {
    if (!writer_) {
      if (waitForPublish) joinWriterPublisher();
      return;
    }
    joinWriterPublisher();
    auto writer = std::move(writer_);
    const auto path = writer->path();
    const auto finalPath = writerFinalPath_;
    chunkStartedAt_.reset();
    currentChunkDurationSeconds_ = 0;
    writerFinalPath_.clear();
    writerWidth_ = 0;
    writerHeight_ = 0;
    writerPublisherThread_ = std::thread([
      this,
      writer = std::move(writer),
      path,
      finalPath
    ]() mutable {
      try {
        writer->finalize();
        writer.reset();
        std::error_code error;
        if (!fs::exists(path, error) || fs::file_size(path, error) == 0) {
          fs::remove(path, error);
          return;
        }
        fs::remove(finalPath, error);
        error.clear();
        fs::rename(path, finalPath, error);
        if (error) {
          fs::remove(path, error);
          throw std::runtime_error("완성된 녹화 청크를 게시하지 못했습니다");
        }
        const auto bytesUsed = ringStorage_.publish(
          finalPath,
          static_cast<std::uint64_t>(options_.capacityGb) * Gigabyte,
          static_cast<std::int64_t>(options_.maxDurationSeconds) * 1000
        );
        std::lock_guard lock(status_.mutex);
        status_.bytesUsed = bytesUsed;
      } catch (const hresult_error& error) {
        failed_ = true;
        std::lock_guard lock(status_.mutex);
        status_.recording = false;
        status_.error = L"녹화 청크 확정 오류: " + std::wstring(error.message());
      } catch (const std::exception&) {
        failed_ = true;
        std::lock_guard lock(status_.mutex);
        status_.recording = false;
        status_.error = L"녹화 청크 파일 게시 오류";
      }
    });
    if (waitForPublish) joinWriterPublisher();
  }

  void startWriter(const FramePacket& packet) {
    fs::create_directories(ringDirectory_);
    const auto identifier = std::to_wstring(epochMilliseconds());
    writerFinalPath_ = ringDirectory_ / (L"chunk-" + identifier + L".mp4");
    const auto output = ringDirectory_
      / (L"chunk-" + identifier + L".partial.mp4");
    writer_ = std::make_unique<Mp4Writer>(
      device_.Get(),
      output,
      packet.width,
      packet.height,
      options_.fps,
      options_.bitrateMbps,
      options_.maxHeight,
      options_.codec
    );
    chunkStartedAt_ = packet.capturedAt;
    currentChunkDurationSeconds_ = 0;
    writerWidth_ = packet.width;
    writerHeight_ = packet.height;
    std::lock_guard lock(status_.mutex);
    status_.recording = true;
    status_.waitingForGame = false;
    status_.width = packet.width;
    status_.height = packet.height;
    status_.error.clear();
  }

  void run() {
    init_apartment(apartment_type::multi_threaded);
    SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
    while (true) {
      FramePacket packet;
      AudioPacket audioPacket;
      bool hasPacket = false;
      bool hasAudioPacket = false;
      bool flush = false;
      int clipSeconds = 0;
      std::uint64_t flushRequestId = 0;
      std::uint64_t clearRequestId = 0;
      {
        std::unique_lock lock(mutex_);
        condition_.wait(lock, [&] {
          return stopping_
            || !queue_.empty()
            || !audioQueue_.empty()
            || flushRequested_;
        });
        if (
          !queue_.empty()
          && (
            audioQueue_.empty()
            || queue_.front().capturedAt <= audioQueue_.front().capturedAt
          )
        ) {
          packet = std::move(queue_.front());
          queue_.pop();
          hasPacket = true;
        } else if (!audioQueue_.empty()) {
          audioPacket = std::move(audioQueue_.front());
          queuedAudioFrames_ -= audioPacket.frameCount;
          audioQueue_.pop();
          hasAudioPacket = true;
        }
        flush = flushRequested_;
        flushRequested_ = false;
        clipSeconds = clipSeconds_;
        clipSeconds_ = 0;
        flushRequestId = flushRequestId_;
        flushRequestId_ = 0;
        clearRequestId = clearRequestId_;
        clearRequestId_ = 0;
        if (stopping_ && !hasPacket && !hasAudioPacket) {
          closeWriter(true);
          break;
        }
      }
      try {
        if (flush) closeWriter(true);
        if (flushRequestId > 0) {
          std::lock_guard lock(status_.mutex);
          status_.flushCompletedId = flushRequestId;
        }
        if (clearRequestId > 0) {
          if (clipThread_.joinable()) clipThread_.join();
          const auto bytesUsed = ringStorage_.clear();
          std::lock_guard lock(status_.mutex);
          status_.bytesUsed = bytesUsed;
          status_.clearCompletedId = clearRequestId;
        }
        if (clipSeconds > 0) {
          if (clipThread_.joinable()) clipThread_.join();
          clipThread_ = std::thread([this, clipSeconds] {
            createClip(ringDirectory_, clipSeconds, status_);
          });
        }
        if (hasAudioPacket) {
          if (writer_ && chunkStartedAt_) {
            const auto elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
              audioPacket.capturedAt - *chunkStartedAt_
            ).count() / 100;
            if (elapsed >= 0) {
              writer_->writeAudio(
                audioPacket.samples,
                audioPacket.frameCount,
                elapsed
              );
            }
          }
          continue;
        }
        if (!hasPacket) continue;
        if (
          writer_
          && (
            packet.width != writerWidth_
            || packet.height != writerHeight_
            || (
              chunkStartedAt_
              && packet.capturedAt - *chunkStartedAt_
                >= std::chrono::seconds(options_.chunkSeconds)
            )
          )
        ) {
          closeWriter();
        }
        bool skipPacket = false;
        if (!writer_) {
          const auto writerStart = std::chrono::steady_clock::now();
          startWriter(packet);
          const auto writerStartupTime = std::chrono::steady_clock::now() - writerStart;
          const auto queueCoverage = std::chrono::microseconds(
            2000000 / std::max(1, options_.fps)
          );
          if (writerStartupTime > queueCoverage) {
            discardQueuedVideoFrames();
            chunkStartedAt_.reset();
            currentChunkDurationSeconds_ = 0;
            skipPacket = true;
            status_.droppedFrames.fetch_add(1);
          }
        }
        if (!skipPacket) {
          if (!chunkStartedAt_) chunkStartedAt_ = packet.capturedAt;
          const auto elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
            packet.capturedAt - *chunkStartedAt_
          ).count() / 100;
          writer_->write(packet.texture.Get(), std::max<LONGLONG>(0, elapsed));
          currentChunkDurationSeconds_ = static_cast<double>(elapsed) / 10000000.0;
        }
      } catch (const hresult_error& error) {
        closeWriter(true);
        failed_ = true;
        std::lock_guard lock(status_.mutex);
        status_.recording = false;
        status_.error = L"녹화 인코더 오류: " + std::wstring(error.message());
      } catch (const std::exception&) {
        closeWriter(true);
        failed_ = true;
        std::lock_guard lock(status_.mutex);
        status_.recording = false;
        status_.error = L"녹화 파일 처리 오류";
      }
      if (hasPacket && packet.texturePool) {
        packet.texturePool->release(std::move(packet.texture));
      }
    }
  }

  ComPtr<ID3D11Device> device_;
  fs::path ringDirectory_;
  RingStorageIndex& ringStorage_;
  Options options_;
  SharedStatus& status_;
  std::mutex mutex_;
  std::condition_variable condition_;
  std::queue<FramePacket> queue_;
  std::queue<AudioPacket> audioQueue_;
  std::size_t queuedAudioFrames_ = 0;
  bool stopping_ = false;
  std::atomic_bool failed_ = false;
  bool flushRequested_ = false;
  int clipSeconds_ = 0;
  std::uint64_t flushRequestId_ = 0;
  std::uint64_t clearRequestId_ = 0;
  std::thread thread_;
  std::thread clipThread_;
  std::thread writerPublisherThread_;
  std::unique_ptr<Mp4Writer> writer_;
  fs::path writerFinalPath_;
  std::optional<std::chrono::steady_clock::time_point> chunkStartedAt_;
  std::atomic<double> currentChunkDurationSeconds_ = 0;
  int writerWidth_ = 0;
  int writerHeight_ = 0;
};

class AudioActivationHandler final
  : public Microsoft::WRL::RuntimeClass<
      Microsoft::WRL::RuntimeClassFlags<Microsoft::WRL::ClassicCom>,
      Microsoft::WRL::FtmBase,
      IActivateAudioInterfaceCompletionHandler
    > {
public:
  AudioActivationHandler()
    : completedEvent_(CreateEventW(nullptr, TRUE, FALSE, nullptr)) {
  }

  ~AudioActivationHandler() {
    if (completedEvent_) CloseHandle(completedEvent_);
  }

  STDMETHODIMP ActivateCompleted(
    IActivateAudioInterfaceAsyncOperation* operation
  ) override {
    ComPtr<IUnknown> activatedInterface;
    HRESULT activationResult = E_FAIL;
    const HRESULT operationResult = operation->GetActivateResult(
      &activationResult,
      &activatedInterface
    );
    result_ = FAILED(operationResult) ? operationResult : activationResult;
    if (SUCCEEDED(result_) && activatedInterface) {
      result_ = activatedInterface.As(&audioClient_);
    }
    SetEvent(completedEvent_);
    return S_OK;
  }

  ComPtr<IAudioClient> waitForClient() {
    if (
      !completedEvent_
      || WaitForSingleObject(completedEvent_, 5000) != WAIT_OBJECT_0
    ) {
      throw std::runtime_error("게임 오디오 장치 연결 시간이 초과되었습니다");
    }
    checkStage(result_, L"게임 오디오 장치 연결");
    return audioClient_;
  }

private:
  HANDLE completedEvent_ = nullptr;
  HRESULT result_ = E_PENDING;
  ComPtr<IAudioClient> audioClient_;
};

class ProcessAudioCapture {
public:
  ProcessAudioCapture(
    DWORD processId,
    EncoderWorker& encoder,
    SharedStatus& status
  ) : processId_(processId),
      encoder_(encoder),
      status_(status),
      stopEvent_(CreateEventW(nullptr, TRUE, FALSE, nullptr)),
      audioEvent_(CreateEventW(nullptr, FALSE, FALSE, nullptr)),
      thread_([this] { run(); }) {
  }

  ~ProcessAudioCapture() {
    stop();
  }

  void stop() {
    if (stopping_.exchange(true)) return;
    if (stopEvent_) SetEvent(stopEvent_);
    if (audioEvent_) SetEvent(audioEvent_);
    if (thread_.joinable()) thread_.join();
    if (stopEvent_) {
      CloseHandle(stopEvent_);
      stopEvent_ = nullptr;
    }
    if (audioEvent_) {
      CloseHandle(audioEvent_);
      audioEvent_ = nullptr;
    }
  }

private:
  ComPtr<IAudioClient> activateAudioClient() {
    AUDIOCLIENT_ACTIVATION_PARAMS activationParameters{};
    activationParameters.ActivationType =
      AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK;
    activationParameters.ProcessLoopbackParams.TargetProcessId = processId_;
    activationParameters.ProcessLoopbackParams.ProcessLoopbackMode =
      PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE;

    PROPVARIANT parameters;
    PropVariantInit(&parameters);
    parameters.vt = VT_BLOB;
    parameters.blob.cbSize = sizeof(activationParameters);
    parameters.blob.pBlobData =
      reinterpret_cast<BYTE*>(&activationParameters);

    auto handler = Microsoft::WRL::Make<AudioActivationHandler>();
    if (!handler) throw std::runtime_error("게임 오디오 연결 객체를 만들지 못했습니다");
    ComPtr<IActivateAudioInterfaceAsyncOperation> operation;
    checkStage(
      ActivateAudioInterfaceAsync(
        VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
        __uuidof(IAudioClient),
        &parameters,
        handler.Get(),
        &operation
      ),
      L"게임 프로세스 오디오 연결 시작"
    );
    return handler->waitForClient();
  }

  void run() {
    init_apartment(apartment_type::multi_threaded);
    DWORD multimediaTaskIndex = 0;
    HANDLE multimediaTask = AvSetMmThreadCharacteristicsW(
      L"Audio",
      &multimediaTaskIndex
    );
    try {
      auto audioClient = activateAudioClient();
      WAVEFORMATEX format{};
      format.wFormatTag = WAVE_FORMAT_PCM;
      format.nChannels = AudioChannels;
      format.nSamplesPerSec = AudioSampleRate;
      format.wBitsPerSample = AudioBitsPerSample;
      format.nBlockAlign = AudioBlockAlignment;
      format.nAvgBytesPerSec = AudioSampleRate * AudioBlockAlignment;

      if (!audioEvent_) throw std::runtime_error("게임 오디오 이벤트를 만들지 못했습니다");
      checkStage(
        audioClient->Initialize(
          AUDCLNT_SHAREMODE_SHARED,
          AUDCLNT_STREAMFLAGS_LOOPBACK
            | AUDCLNT_STREAMFLAGS_EVENTCALLBACK
            | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
            | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
          0,
          0,
          &format,
          nullptr
        ),
        L"게임 프로세스 오디오 캡처 초기화"
      );
      checkStage(audioClient->SetEventHandle(audioEvent_), L"게임 오디오 이벤트 연결");
      ComPtr<IAudioCaptureClient> captureClient;
      checkStage(
        audioClient->GetService(IID_PPV_ARGS(&captureClient)),
        L"게임 오디오 캡처 서비스 연결"
      );
      checkStage(audioClient->Start(), L"게임 오디오 캡처 시작");
      {
        std::lock_guard lock(status_.mutex);
        status_.audioRecording = true;
        status_.audioError.clear();
        status_.audioGainDb = 0;
      }

      HANDLE events[] = { stopEvent_, audioEvent_ };
      std::optional<std::chrono::steady_clock::time_point> audioCursor;
      while (!stopping_) {
        const DWORD waitResult = WaitForMultipleObjects(2, events, FALSE, 250);
        if (waitResult == WAIT_OBJECT_0) break;
        if (waitResult != WAIT_OBJECT_0 + 1) continue;

        UINT32 nextPacketFrames = 0;
        checkStage(
          captureClient->GetNextPacketSize(&nextPacketFrames),
          L"게임 오디오 패킷 크기 확인"
        );
        while (nextPacketFrames > 0 && !stopping_) {
          BYTE* source = nullptr;
          UINT32 frameCount = 0;
          DWORD flags = 0;
          UINT64 devicePosition = 0;
          UINT64 qpcPosition = 0;
          checkStage(
            captureClient->GetBuffer(
              &source,
              &frameCount,
              &flags,
              &devicePosition,
              &qpcPosition
            ),
            L"게임 오디오 패킷 읽기"
          );
          AudioPacket packet;
          packet.frameCount = frameCount;
          packet.samples.resize(
            static_cast<std::size_t>(frameCount) * AudioBlockAlignment
          );
          if (
            !(flags & AUDCLNT_BUFFERFLAGS_SILENT)
            && source
            && !packet.samples.empty()
          ) {
            std::memcpy(packet.samples.data(), source, packet.samples.size());
          }
          const double gainDb = audioGainController_.process(
            packet.samples,
            frameCount
          );
          if (std::abs(gainDb - lastReportedGainDb_) >= 0.25) {
            std::lock_guard lock(status_.mutex);
            status_.audioGainDb = gainDb;
            lastReportedGainDb_ = gainDb;
          }
          const auto packetDuration = std::chrono::nanoseconds(
            static_cast<std::int64_t>(frameCount) * 1000000000ll / AudioSampleRate
          );
          const auto now = std::chrono::steady_clock::now();
          const bool timestampInvalid =
            (flags & AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR) || qpcPosition == 0;
          auto capturedAt = timestampInvalid
            ? audioCursor.value_or(now - packetDuration)
            : steadyTimeFromQpc100Nanoseconds(qpcPosition);
          if (
            audioCursor
            && (
              (flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY)
              || capturedAt + 50ms < *audioCursor
              || capturedAt > *audioCursor + 250ms
            )
          ) {
            capturedAt = *audioCursor;
          }
          packet.capturedAt = capturedAt;
          audioCursor = capturedAt + packetDuration;
          encoder_.enqueueAudio(std::move(packet));
          checkStage(
            captureClient->ReleaseBuffer(frameCount),
            L"게임 오디오 패킷 반환"
          );
          checkStage(
            captureClient->GetNextPacketSize(&nextPacketFrames),
            L"다음 게임 오디오 패킷 확인"
          );
        }
      }
      audioClient->Stop();
    } catch (const hresult_error& error) {
      std::lock_guard lock(status_.mutex);
      std::wostringstream code;
      code << L"0x" << std::hex << std::uppercase
        << static_cast<std::uint32_t>(error.code());
      status_.audioError = L"게임 소리 녹음 실패: "
        + std::wstring(error.message())
        + L" (" + code.str() + L")";
    } catch (const std::exception& error) {
      std::lock_guard lock(status_.mutex);
      const int required = MultiByteToWideChar(
        CP_UTF8,
        0,
        error.what(),
        -1,
        nullptr,
        0
      );
      std::wstring message(static_cast<std::size_t>(std::max(0, required)), L'\0');
      if (required > 1) {
        MultiByteToWideChar(
          CP_UTF8,
          0,
          error.what(),
          -1,
          message.data(),
          required
        );
        message.resize(static_cast<std::size_t>(required - 1));
      }
      status_.audioError = L"게임 소리 녹음 실패: " + message;
    }
    {
      std::lock_guard lock(status_.mutex);
      status_.audioRecording = false;
      status_.audioGainDb = 0;
    }
    if (multimediaTask) AvRevertMmThreadCharacteristics(multimediaTask);
  }

  DWORD processId_;
  EncoderWorker& encoder_;
  SharedStatus& status_;
  HANDLE stopEvent_ = nullptr;
  HANDLE audioEvent_ = nullptr;
  std::atomic_bool stopping_ = false;
  AudioGainController audioGainController_;
  double lastReportedGainDb_ = -1000;
  std::thread thread_;
};

class WindowCapture {
public:
  WindowCapture(
    HWND window,
    IDirect3DDevice winrtDevice,
    ComPtr<ID3D11Device> device,
    ComPtr<ID3D11DeviceContext> context,
    EncoderWorker& encoder,
    SharedStatus& status,
    int fps
  ) : window_(window),
      winrtDevice_(winrtDevice),
      device_(std::move(device)),
      context_(std::move(context)),
      encoder_(encoder),
      status_(status),
      minimumFrameInterval_(1000000 / std::max(1, fps)) {
    item_ = createCaptureItem(window);
    const auto size = item_.Size();
    frameSize_ = size;
    framePool_ = Direct3D11CaptureFramePool::CreateFreeThreaded(
      winrtDevice,
      DirectXPixelFormat::B8G8R8A8UIntNormalized,
      2,
      size
    );
    session_ = framePool_.CreateCaptureSession(item_);
    frameToken_ = framePool_.FrameArrived({ this, &WindowCapture::onFrame });
    closedToken_ = item_.Closed([this](auto&&, auto&&) { closed_ = true; });
    try {
      session_.IsCursorCaptureEnabled(false);
    } catch (...) {
    }
    session_.StartCapture();
  }

  ~WindowCapture() {
    stop();
  }

  bool closed() const {
    return closed_ || !IsWindow(window_);
  }

  void stop() {
    if (stopped_.exchange(true)) return;
    if (framePool_) framePool_.FrameArrived(frameToken_);
    if (item_) item_.Closed(closedToken_);
    std::lock_guard callbackLock(callbackMutex_);
    if (session_) session_.Close();
    if (framePool_) framePool_.Close();
    encoder_.flush();
  }

private:
  void onFrame(
    Direct3D11CaptureFramePool const& sender,
    winrt::Windows::Foundation::IInspectable const&
  ) {
    std::lock_guard callbackLock(callbackMutex_);
    if (stopped_) return;
    auto frame = sender.TryGetNextFrame();
    if (!frame) return;
    const auto capturedAt = std::chrono::steady_clock::now();
    if (
      nextFrameAt_
      && capturedAt + 1ms < *nextFrameAt_
    ) return;
    if (!nextFrameAt_) {
      nextFrameAt_ = capturedAt + minimumFrameInterval_;
    } else {
      *nextFrameAt_ += minimumFrameInterval_;
      if (*nextFrameAt_ < capturedAt) {
        nextFrameAt_ = capturedAt + minimumFrameInterval_;
      }
    }
    const auto contentSize = frame.ContentSize();
    using DxgiAccess =
      ::Windows::Graphics::DirectX::Direct3D11::IDirect3DDxgiInterfaceAccess;
    ComPtr<DxgiAccess> access;
    auto surface = frame.Surface();
    if (FAILED(reinterpret_cast<IUnknown*>(get_abi(surface))->QueryInterface(
      IID_PPV_ARGS(&access)
    ))) return;
    ComPtr<ID3D11Texture2D> source;
    if (FAILED(access->GetInterface(IID_PPV_ARGS(&source)))) return;
    D3D11_TEXTURE2D_DESC description{};
    source->GetDesc(&description);
    if (description.Width == 0 || description.Height == 0) return;
    description.BindFlags = D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET;
    description.CPUAccessFlags = 0;
    description.MiscFlags = 0;
    description.Usage = D3D11_USAGE_DEFAULT;
    description.ArraySize = 1;
    description.MipLevels = 1;
    auto copy = texturePool_->acquire(device_.Get(), description);
    if (!copy) {
      status_.droppedFrames.fetch_add(1);
      return;
    }
    context_->CopyResource(copy.Get(), source.Get());
    FramePacket packet{
      std::move(copy),
      texturePool_,
      capturedAt,
      static_cast<int>(description.Width),
      static_cast<int>(description.Height)
    };
    if (!encoder_.enqueue(std::move(packet))) {
      status_.droppedFrames.fetch_add(1);
      texturePool_->release(std::move(packet.texture));
    }
    if (
      contentSize.Width > 0
      && contentSize.Height > 0
      && (contentSize.Width != frameSize_.Width || contentSize.Height != frameSize_.Height)
    ) {
      frameSize_ = contentSize;
      sender.Recreate(
        winrtDevice_,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        2,
        frameSize_
      );
    }
  }

  HWND window_;
  IDirect3DDevice winrtDevice_{ nullptr };
  ComPtr<ID3D11Device> device_;
  ComPtr<ID3D11DeviceContext> context_;
  EncoderWorker& encoder_;
  SharedStatus& status_;
  GraphicsCaptureItem item_{ nullptr };
  Direct3D11CaptureFramePool framePool_{ nullptr };
  GraphicsCaptureSession session_{ nullptr };
  event_token frameToken_{};
  event_token closedToken_{};
  SizeInt32 frameSize_{};
  std::mutex callbackMutex_;
  std::shared_ptr<TexturePool> texturePool_ = std::make_shared<TexturePool>();
  std::chrono::microseconds minimumFrameInterval_;
  std::optional<std::chrono::steady_clock::time_point> nextFrameAt_;
  std::atomic_bool closed_ = false;
  std::atomic_bool stopped_ = false;
};

void updateStatusFile(const Options& options, SharedStatus& status) {
  SharedStatus snapshot;
  {
    std::lock_guard lock(status.mutex);
    snapshot.running = status.running;
    snapshot.recording = status.recording;
    snapshot.waitingForGame = status.waitingForGame;
    snapshot.clipInProgress = status.clipInProgress;
    snapshot.audioRecording = status.audioRecording;
    snapshot.codec = status.codec;
    snapshot.error = status.error;
    snapshot.audioError = status.audioError;
    snapshot.latestClip = status.latestClip;
    snapshot.audioGainDb = status.audioGainDb;
    snapshot.durationSeconds = status.durationSeconds;
    snapshot.bytesUsed = status.bytesUsed;
    snapshot.capacityBytes = status.capacityBytes;
    snapshot.droppedFrames = status.droppedFrames.load();
    snapshot.flushCompletedId = status.flushCompletedId;
    snapshot.clearCompletedId = status.clearCompletedId;
    snapshot.width = status.width;
    snapshot.height = status.height;
    snapshot.fps = status.fps;
  }
  std::ostringstream output;
  output
    << "{"
    << "\"running\":" << (snapshot.running ? "true" : "false") << ","
    << "\"recording\":" << (snapshot.recording ? "true" : "false") << ","
    << "\"waitingForGame\":" << (snapshot.waitingForGame ? "true" : "false") << ","
    << "\"clipInProgress\":" << (snapshot.clipInProgress ? "true" : "false") << ","
    << "\"audioRecording\":" << (snapshot.audioRecording ? "true" : "false") << ","
    << "\"codec\":\"" << jsonEscape(snapshot.codec) << "\","
    << "\"error\":" << (snapshot.error.empty()
      ? "null"
      : "\"" + jsonEscape(snapshot.error) + "\"") << ","
    << "\"audioError\":" << (snapshot.audioError.empty()
      ? "null"
      : "\"" + jsonEscape(snapshot.audioError) + "\"") << ","
    << "\"audioGainDb\":" << std::fixed << std::setprecision(2)
      << snapshot.audioGainDb << ","
    << "\"durationSeconds\":" << std::fixed << std::setprecision(1)
      << snapshot.durationSeconds << ","
    << "\"latestClip\":" << (snapshot.latestClip.empty()
      ? "null"
      : "\"" + jsonEscape(snapshot.latestClip) + "\"") << ","
    << "\"bytesUsed\":" << snapshot.bytesUsed << ","
    << "\"capacityBytes\":" << snapshot.capacityBytes << ","
    << "\"droppedFrames\":" << snapshot.droppedFrames.load() << ","
    << "\"flushCompletedId\":" << snapshot.flushCompletedId << ","
    << "\"clearCompletedId\":" << snapshot.clearCompletedId << ","
    << "\"width\":" << snapshot.width << ","
    << "\"height\":" << snapshot.height << ","
    << "\"fps\":" << snapshot.fps << ","
    << "\"pid\":" << GetCurrentProcessId() << ","
    << "\"updatedAt\":" << epochMilliseconds()
    << "}";
  writeTextAtomic(options.statusPath, output.str());
}

struct ControlCommand {
  std::string command;
  int seconds = 0;
  std::uint64_t requestId = 0;
};

std::optional<ControlCommand> readControl(const fs::path& path) {
  const auto text = readText(path);
  if (text.empty()) return std::nullopt;
  std::error_code error;
  fs::remove(path, error);
  std::smatch commandMatch;
  if (!std::regex_search(
    text,
    commandMatch,
    std::regex(R"json("command"\s*:\s*"([^"]+)")json")
  )) return std::nullopt;
  int seconds = 0;
  std::smatch secondsMatch;
  if (std::regex_search(text, secondsMatch, std::regex(R"("seconds"\s*:\s*(\d+))"))) {
    seconds = std::clamp(std::stoi(secondsMatch[1].str()), 5, 600);
  }
  std::uint64_t requestId = 0;
  std::smatch requestIdMatch;
  if (std::regex_search(
    text,
    requestIdMatch,
    std::regex(R"("requestId"\s*:\s*(\d+))")
  )) {
    requestId = std::stoull(requestIdMatch[1].str());
  }
  return ControlCommand{ commandMatch[1].str(), seconds, requestId };
}

std::int64_t wideInteger(
  const std::map<std::wstring, std::wstring>& arguments,
  const std::wstring& name,
  std::int64_t fallback
) {
  const auto iterator = arguments.find(name);
  if (iterator == arguments.end()) return fallback;
  try {
    return std::stoll(iterator->second);
  } catch (...) {
    return fallback;
  }
}

struct CompositionPiece {
  bool black;
  LONGLONG start;
  LONGLONG duration;
};

std::vector<CompositionPiece> parseCompositionPieces(const std::wstring& value) {
  std::vector<CompositionPiece> pieces;
  std::wstringstream stream(value);
  std::wstring token;
  while (std::getline(stream, token, L';')) {
    std::vector<std::wstring> parts;
    std::wstringstream tokenStream(token);
    std::wstring part;
    while (std::getline(tokenStream, part, L':')) parts.push_back(part);
    try {
      if (parts.size() == 2 && parts[0] == L"black") {
        const auto duration = std::stoll(parts[1]) * 10000ll;
        if (duration > 0) pieces.push_back({ true, 0, duration });
      } else if (parts.size() == 3 && parts[0] == L"media") {
        const auto start = std::max<LONGLONG>(0, std::stoll(parts[1]) * 10000ll);
        const auto duration = std::stoll(parts[2]) * 10000ll;
        if (duration > 0) pieces.push_back({ false, start, duration });
      }
    } catch (...) {
      throw std::runtime_error("영상 추출 구간 정보가 올바르지 않습니다");
    }
  }
  if (pieces.empty()) throw std::runtime_error("영상 추출 구간이 비어 있습니다");
  return pieces;
}

void createBlackVideo(
  const fs::path& referencePath,
  const fs::path& outputPath,
  LONGLONG duration
) {
  const auto signature = mediaSignature(referencePath);
  ComPtr<IMFAttributes> attributes;
  check_hresult(MFCreateAttributes(&attributes, 2));
  check_hresult(attributes->SetUINT32(MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, TRUE));
  check_hresult(attributes->SetUINT32(MF_SINK_WRITER_DISABLE_THROTTLING, TRUE));
  ComPtr<IMFSinkWriter> writer;
  check_hresult(MFCreateSinkWriterFromURL(
    outputPath.c_str(),
    nullptr,
    attributes.Get(),
    &writer
  ));

  ComPtr<IMFMediaType> videoOutputType;
  check_hresult(MFCreateMediaType(&videoOutputType));
  check_hresult(videoOutputType->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video));
  check_hresult(videoOutputType->SetGUID(MF_MT_SUBTYPE, signature.subtype));
  check_hresult(videoOutputType->SetUINT32(
    MF_MT_AVG_BITRATE,
    signature.averageBitrate
  ));
  check_hresult(videoOutputType->SetUINT32(
    MF_MT_INTERLACE_MODE,
    MFVideoInterlace_Progressive
  ));
  check_hresult(MFSetAttributeSize(
    videoOutputType.Get(),
    MF_MT_FRAME_SIZE,
    signature.width,
    signature.height
  ));
  check_hresult(MFSetAttributeRatio(
    videoOutputType.Get(),
    MF_MT_FRAME_RATE,
    signature.frameRateNumerator,
    signature.frameRateDenominator
  ));
  check_hresult(MFSetAttributeRatio(
    videoOutputType.Get(),
    MF_MT_PIXEL_ASPECT_RATIO,
    1,
    1
  ));
  DWORD videoStream = 0;
  check_hresult(writer->AddStream(videoOutputType.Get(), &videoStream));

  ComPtr<IMFMediaType> videoInputType;
  check_hresult(MFCreateMediaType(&videoInputType));
  check_hresult(videoInputType->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video));
  check_hresult(videoInputType->SetGUID(MF_MT_SUBTYPE, MFVideoFormat_NV12));
  check_hresult(videoInputType->SetUINT32(
    MF_MT_INTERLACE_MODE,
    MFVideoInterlace_Progressive
  ));
  check_hresult(videoInputType->SetUINT32(MF_MT_DEFAULT_STRIDE, signature.width));
  check_hresult(MFSetAttributeSize(
    videoInputType.Get(),
    MF_MT_FRAME_SIZE,
    signature.width,
    signature.height
  ));
  check_hresult(MFSetAttributeRatio(
    videoInputType.Get(),
    MF_MT_FRAME_RATE,
    signature.frameRateNumerator,
    signature.frameRateDenominator
  ));
  check_hresult(MFSetAttributeRatio(
    videoInputType.Get(),
    MF_MT_PIXEL_ASPECT_RATIO,
    1,
    1
  ));
  check_hresult(writer->SetInputMediaType(
    videoStream,
    videoInputType.Get(),
    nullptr
  ));

  std::optional<DWORD> audioStream;
  if (signature.hasAudio) {
    DWORD stream = 0;
    const auto audioOutputType = createAacAudioType();
    check_hresult(writer->AddStream(audioOutputType.Get(), &stream));
    const auto audioInputType = createPcmAudioType();
    check_hresult(writer->SetInputMediaType(
      stream,
      audioInputType.Get(),
      nullptr
    ));
    audioStream = stream;
  }
  check_hresult(writer->BeginWriting());

  const DWORD yPlaneBytes = signature.width * signature.height;
  const DWORD videoBytes = yPlaneBytes * 3 / 2;
  ComPtr<IMFMediaBuffer> videoBuffer;
  check_hresult(MFCreateMemoryBuffer(videoBytes, &videoBuffer));
  BYTE* videoData = nullptr;
  check_hresult(videoBuffer->Lock(&videoData, nullptr, nullptr));
  std::memset(videoData, 16, yPlaneBytes);
  std::memset(videoData + yPlaneBytes, 128, videoBytes - yPlaneBytes);
  check_hresult(videoBuffer->Unlock());
  check_hresult(videoBuffer->SetCurrentLength(videoBytes));
  ComPtr<IMFSample> videoSample;
  check_hresult(MFCreateSample(&videoSample));
  check_hresult(videoSample->AddBuffer(videoBuffer.Get()));
  check_hresult(videoSample->SetSampleTime(0));
  check_hresult(videoSample->SetSampleDuration(duration));
  check_hresult(writer->WriteSample(videoStream, videoSample.Get()));

  if (audioStream) {
    constexpr UINT32 audioFrames = 1024;
    const LONGLONG audioDuration =
      static_cast<LONGLONG>(audioFrames) * 10000000ll / AudioSampleRate;
    const DWORD audioBytes = audioFrames * AudioBlockAlignment;
    for (LONGLONG timestamp = 0; timestamp < duration; timestamp += audioDuration) {
      ComPtr<IMFMediaBuffer> audioBuffer;
      check_hresult(MFCreateMemoryBuffer(audioBytes, &audioBuffer));
      BYTE* audioData = nullptr;
      check_hresult(audioBuffer->Lock(&audioData, nullptr, nullptr));
      std::memset(audioData, 0, audioBytes);
      check_hresult(audioBuffer->Unlock());
      check_hresult(audioBuffer->SetCurrentLength(audioBytes));
      ComPtr<IMFSample> audioSample;
      check_hresult(MFCreateSample(&audioSample));
      check_hresult(audioSample->AddBuffer(audioBuffer.Get()));
      check_hresult(audioSample->SetSampleTime(timestamp));
      check_hresult(audioSample->SetSampleDuration(
        std::min(audioDuration, duration - timestamp)
      ));
      check_hresult(writer->WriteSample(*audioStream, audioSample.Get()));
    }
  }
  check_hresult(writer->Finalize());
}

void composeExtraction(
  const std::vector<fs::path>& inputPaths,
  const fs::path& outputPath,
  const std::vector<CompositionPiece>& pieces,
  double playbackRate
) {
  if (inputPaths.empty()) {
    throw std::runtime_error("추출할 녹화 청크가 없습니다");
  }
  if (
    pieces.size() == 1
    && !pieces.front().black
    && std::abs(playbackRate - 1.0) < 0.001
  ) {
    int lastProgress = -1;
    if (!remuxChunksAtomically(
      inputPaths,
      outputPath,
      pieces.front().start,
      pieces.front().duration,
      false,
      1.0,
      [&lastProgress](std::size_t completed, std::size_t total) {
        const auto progress = static_cast<int>(
          static_cast<double>(completed) * 100.0
          / std::max<std::size_t>(1, total)
        );
        if (progress <= lastProgress) return;
        lastProgress = progress;
        std::cerr << "PROGRESS " << progress << '\n';
      }
    )) {
      throw std::runtime_error("선택한 녹화 구간을 저장하지 못했습니다");
    }
    std::cerr << "PROGRESS 100\n";
    return;
  }
  const auto stagingDirectory = outputPath.parent_path()
    / (L".compose-" + std::to_wstring(epochMilliseconds()));
  fs::create_directories(stagingDirectory);
  std::vector<fs::path> partPaths;
  try {
    for (std::size_t index = 0; index < pieces.size(); ++index) {
      const auto partPath = stagingDirectory
        / (L"part-" + std::to_wstring(index) + L".mp4");
      const auto& piece = pieces[index];
      if (piece.black) {
        createBlackVideo(inputPaths.front(), partPath, piece.duration);
      } else if (!remuxChunksAtomically(
        inputPaths,
        partPath,
        piece.start,
        piece.duration,
        false,
        1.0,
        [index, &pieces, lastProgress = -1](
          std::size_t completed,
          std::size_t total
        ) mutable {
          const auto pieceProgress = static_cast<double>(completed)
            / std::max<std::size_t>(1, total);
          const auto overallProgress = static_cast<int>(
            (static_cast<double>(index) + pieceProgress)
            * 80.0
            / std::max<std::size_t>(1, pieces.size())
          );
          if (overallProgress <= lastProgress) return;
          lastProgress = overallProgress;
          std::cerr << "PROGRESS " << overallProgress << '\n';
        }
      )) {
        throw std::runtime_error("녹화된 영상 구간을 준비하지 못했습니다");
      }
      partPaths.push_back(partPath);
      const int progress = static_cast<int>(
        (index + 1) * 80 / std::max<std::size_t>(1, pieces.size())
      );
      std::cerr << "PROGRESS " << progress << '\n';
    }
    if (!remuxChunksAtomically(
      partPaths,
      outputPath,
      0,
      LLONG_MAX,
      true,
      playbackRate
    )) {
      throw std::runtime_error("검은 공백을 포함한 영상을 결합하지 못했습니다");
    }
    std::cerr << "PROGRESS 100\n";
    fs::remove_all(stagingDirectory);
  } catch (...) {
    std::error_code error;
    fs::remove_all(stagingDirectory, error);
    throw;
  }
}

int runUtilityMode(const std::map<std::wstring, std::wstring>& arguments) {
  const auto mode = arguments.at(L"mode");
  init_apartment(apartment_type::multi_threaded);
  check_hresult(MFStartup(MF_VERSION, MFSTARTUP_FULL));
  if (!SetPriorityClass(GetCurrentProcess(), PROCESS_MODE_BACKGROUND_BEGIN)) {
    throw std::runtime_error("영상 작업 백그라운드 우선순위를 적용하지 못했습니다");
  }
  try {
    if (mode == L"track" || mode == L"index") {
      const fs::path ringPath = arguments.at(L"ring-path");
      const auto anchorMilliseconds = wideInteger(
        arguments,
        L"anchor-ms",
        epochMilliseconds()
      );
      const int seconds = std::clamp(
        static_cast<int>(wideInteger(arguments, L"seconds", 900)),
        30,
        21600
      );
      const auto chunks = compatibleChunkSuffix(selectAnchoredChunks(
          ringPath,
          anchorMilliseconds,
          seconds
      ));
      if (chunks.empty() && mode == L"track") {
        throw std::runtime_error("편집할 녹화 청크가 없습니다");
      }
      const auto timelineMetadata = trackTimelineMetadata(
        chunks,
        anchorMilliseconds,
        seconds
      );
      if (mode == L"track") {
        const fs::path outputPath = arguments.at(L"output");
        fs::create_directories(outputPath.parent_path());
        if (!remuxChunksAtomically(
          chunks,
          outputPath,
          0,
          timelineMetadata.mediaDuration,
          false,
          1.0
        )) {
          throw std::runtime_error("편집 트랙을 만들지 못했습니다");
        }
      }
      std::cout << timelineMetadata.json << '\n';
    } else if (mode == L"summary") {
      RingStorageIndex ringStorage(arguments.at(L"ring-path"));
      std::cout
        << "{\"bytesUsed\":" << ringStorage.bytesUsed()
        << ",\"durationSeconds\":" << std::fixed << std::setprecision(3)
        << ringStorage.durationSeconds()
        << "}\n";
    } else if (mode == L"compose") {
      const fs::path outputPath = arguments.at(L"output");
      const auto pieces = parseCompositionPieces(arguments.at(L"pieces"));
      const auto playbackRate = static_cast<double>(std::clamp<std::int64_t>(
        wideInteger(arguments, L"speed-milli", 1000),
        250,
        4000
      )) / 1000.0;
      std::vector<fs::path> inputs;
      const auto input = arguments.find(L"input");
      if (input != arguments.end()) {
        inputs.push_back(input->second);
      } else {
        const fs::path ringPath = arguments.at(L"ring-path");
        const auto anchorMilliseconds = wideInteger(
          arguments,
          L"anchor-ms",
          epochMilliseconds()
        );
        const int seconds = std::clamp(
          static_cast<int>(wideInteger(arguments, L"seconds", 900)),
          30,
          21600
        );
        inputs = compatibleChunkSuffix(selectAnchoredChunks(
          ringPath,
          anchorMilliseconds,
          seconds
        ));
      }
      fs::create_directories(outputPath.parent_path());
      composeExtraction(inputs, outputPath, pieces, playbackRate);
    } else if (mode == L"extract") {
      const fs::path inputPath = arguments.at(L"input");
      const fs::path outputPath = arguments.at(L"output");
      const auto startMilliseconds = std::max<std::int64_t>(
        0,
        wideInteger(arguments, L"start-ms", 0)
      );
      const auto durationMilliseconds = std::clamp<std::int64_t>(
        wideInteger(arguments, L"duration-ms", 30000),
        1000,
        21600000
      );
      fs::create_directories(outputPath.parent_path());
      if (!remuxChunksAtomically(
        { inputPath },
        outputPath,
        startMilliseconds * 10000,
        durationMilliseconds * 10000
      )) {
        throw std::runtime_error("선택 구간을 추출하지 못했습니다");
      }
    } else {
      throw std::runtime_error("지원하지 않는 recorder helper 모드입니다");
    }
    MFShutdown();
    return 0;
  } catch (...) {
    MFShutdown();
    throw;
  }
}

} 

int wmain(int count, wchar_t** values) {
  const auto arguments = parseArguments(count, values);
  const auto affinityIterator = arguments.find(L"affinity-mask");
  if (affinityIterator != arguments.end()) {
    try {
      const auto mask = std::stoull(affinityIterator->second, nullptr, 0);
      if (
        mask == 0
        || !SetProcessAffinityMask(GetCurrentProcess(), static_cast<DWORD_PTR>(mask))
      ) {
        std::cerr << "녹화 CPU 코어 격리를 적용하지 못했습니다\n";
        return 1;
      }
    } catch (...) {
      std::cerr << "녹화 CPU 코어 격리 값이 올바르지 않습니다\n";
      return 1;
    }
  }
  if (
    arguments.count(L"mode")
    && (
      arguments.at(L"mode") == L"track"
      || arguments.at(L"mode") == L"index"
      || arguments.at(L"mode") == L"compose"
      || arguments.at(L"mode") == L"extract"
    )
  ) {
    try {
      return runUtilityMode(arguments);
    } catch (const hresult_error& error) {
      std::cerr << utf8(error.message().c_str()) << "\n";
    } catch (const std::exception& error) {
      std::cerr << error.what() << "\n";
    }
    return 1;
  }
  Options options;
  SharedStatus status;
  HANDLE instanceMutex = nullptr;
  try {
    options = optionsFromArguments(count, values);
    instanceMutex = CreateMutexW(
      nullptr,
      TRUE,
      L"Local\\NogiremMabinogiRecorderHelper"
    );
    if (!instanceMutex || GetLastError() == ERROR_ALREADY_EXISTS) {
      throw std::runtime_error("블랙박스 녹화 helper가 이미 실행 중입니다");
    }
    status.codec = options.codec;
    status.capacityBytes = static_cast<std::uint64_t>(options.capacityGb) * Gigabyte;
    status.fps = options.fps;
    if (!SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS)) {
      throw std::runtime_error("녹화 프로세스 우선순위를 제한하지 못했습니다");
    }
    init_apartment(apartment_type::multi_threaded);
    check_hresult(MFStartup(MF_VERSION, MFSTARTUP_FULL));
    requireHardwareVideoEncoder(options.codec);
    fs::create_directories(options.storagePath / L"Ring");
    fs::create_directories(options.storagePath / L"Clips");
    removeIncompleteChunks(options.storagePath / L"Ring");
    RingStorageIndex ringStorage(options.storagePath / L"Ring");
    status.bytesUsed = ringStorage.bytesUsed();
    status.durationSeconds = ringStorage.durationSeconds();

    ComPtr<ID3D11Device> device;
    ComPtr<ID3D11DeviceContext> context;
    const auto winrtDevice = createWinrtDevice(device, context);
    EncoderWorker encoder(
      device,
      options.storagePath / L"Ring",
      ringStorage,
      options,
      status
    );
    std::unique_ptr<WindowCapture> capture;
    std::unique_ptr<ProcessAudioCapture> audioCapture;
    auto lastStatusWrite = std::chrono::steady_clock::now() - 2s;
    auto nextCaptureAttempt = std::chrono::steady_clock::now();
    auto captureRetryDelay = 500ms;

    while (status.running && processRunning(options.parentPid)) {
      const auto now = std::chrono::steady_clock::now();
      if (const auto control = readControl(options.controlPath)) {
        if (control->command == "stop") {
          status.running = false;
          break;
        }
        if (control->command == "clip") encoder.requestClip(control->seconds);
        if (control->command == "flush") encoder.requestFlush(control->requestId);
        if (control->command == "clear") encoder.requestClear(control->requestId);
      }

      if (capture && capture->closed()) {
        audioCapture.reset();
        capture.reset();
        std::lock_guard lock(status.mutex);
        status.recording = false;
        status.audioRecording = false;
        status.waitingForGame = true;
        nextCaptureAttempt = now + 500ms;
        captureRetryDelay = 500ms;
      }
      if (encoder.failed()) {
        status.running = false;
        break;
      }
      if (!capture && now >= nextCaptureAttempt) {
        const HWND window = findGameWindow(options.gamePath);
        if (window) {
          try {
            auto newCapture = std::make_unique<WindowCapture>(
              window,
              winrtDevice,
              device,
              context,
              encoder,
              status,
              options.fps
            );
            DWORD gameProcessId = 0;
            GetWindowThreadProcessId(window, &gameProcessId);
            std::unique_ptr<ProcessAudioCapture> newAudioCapture;
            if (gameProcessId != 0) {
              newAudioCapture = std::make_unique<ProcessAudioCapture>(
                gameProcessId,
                encoder,
                status
              );
            }
            capture = std::move(newCapture);
            audioCapture = std::move(newAudioCapture);
            captureRetryDelay = 500ms;
            std::lock_guard lock(status.mutex);
            status.waitingForGame = false;
            status.error.clear();
          } catch (const hresult_error& error) {
            audioCapture.reset();
            capture.reset();
            nextCaptureAttempt = now + captureRetryDelay;
            captureRetryDelay = std::min(captureRetryDelay * 2, 5000ms);
            std::lock_guard lock(status.mutex);
            status.error = L"게임 화면 캡처 시작 실패: " + std::wstring(error.message());
          } catch (const std::exception&) {
            audioCapture.reset();
            capture.reset();
            nextCaptureAttempt = now + captureRetryDelay;
            captureRetryDelay = std::min(captureRetryDelay * 2, 5000ms);
            std::lock_guard lock(status.mutex);
            status.error = L"게임 화면 캡처 시작 실패";
          }
        } else {
          nextCaptureAttempt = now + 500ms;
        }
      }

      if (now - lastStatusWrite >= 1s) {
        {
          std::lock_guard lock(status.mutex);
          status.durationSeconds =
            ringStorage.durationSeconds() + encoder.currentChunkDurationSeconds();
        }
        try {
          updateStatusFile(options, status);
        } catch (...) {
        }
        lastStatusWrite = now;
      }
      std::this_thread::sleep_for(100ms);
    }

    audioCapture.reset();
    capture.reset();
    encoder.stop();
    {
      std::lock_guard lock(status.mutex);
      status.running = false;
      status.recording = false;
      status.audioRecording = false;
    }
    try {
      updateStatusFile(options, status);
    } catch (...) {
    }
    MFShutdown();
    CloseHandle(instanceMutex);
    return 0;
  } catch (const hresult_error& error) {
    status.running = false;
    status.recording = false;
    status.error = std::wstring(error.message());
  } catch (const std::exception&) {
    status.running = false;
    status.recording = false;
    status.error = L"녹화 helper를 시작하지 못했습니다";
  }
  if (!options.statusPath.empty()) {
    try {
      updateStatusFile(options, status);
    } catch (...) {
    }
  }
  MFShutdown();
  if (instanceMutex) CloseHandle(instanceMutex);
  return 1;
}
