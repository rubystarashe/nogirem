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
#include <filesystem>
#include <fstream>
#include <functional>
#include <iomanip>
#include <iostream>
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
  int chunkSeconds = 4;
  int capacityGb = 50;
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
  std::uint64_t droppedFrames = 0;
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
  options.chunkSeconds = integerArgument(arguments, L"chunk-seconds", 4, 2, 30);
  options.capacityGb = integerArgument(arguments, L"capacity-gb", 50, 1, 4096);
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

std::uint64_t directoryBytes(const fs::path& directory) {
  std::uint64_t total = 0;
  for (const auto& path : completedChunks(directory)) {
    std::error_code error;
    total += fs::file_size(path, error);
  }
  return total;
}

void pruneRing(const fs::path& directory, std::uint64_t capacityBytes) {
  auto chunks = completedChunks(directory);
  std::uint64_t total = 0;
  for (const auto& path : chunks) {
    std::error_code error;
    total += fs::file_size(path, error);
  }
  std::error_code spaceError;
  auto space = fs::space(directory, spaceError);
  const std::uint64_t reserve = std::max<std::uint64_t>(5 * Gigabyte, capacityBytes / 10);
  for (const auto& path : chunks) {
    const bool capacityExceeded = total > capacityBytes;
    const bool reserveExceeded = !spaceError && space.available < reserve;
    if (!capacityExceeded && !reserveExceeded) break;
    std::error_code sizeError;
    const auto size = fs::file_size(path, sizeError);
    std::error_code removeError;
    if (fs::remove(path, removeError)) {
      if (!sizeError && total >= size) total -= size;
      if (!spaceError) space.available += size;
    }
  }
}

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
    check_hresult(videoDevice_->CreateVideoProcessorEnumerator(&content, &enumerator_));
    check_hresult(videoDevice_->CreateVideoProcessor(enumerator_.Get(), 0, &processor_));
    videoContext_->VideoProcessorSetStreamFrameFormat(
      processor_.Get(),
      0,
      D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE
    );
  }

  ComPtr<ID3D11Texture2D> convert(ID3D11Texture2D* source) {
    D3D11_TEXTURE2D_DESC outputDescription{};
    outputDescription.Width = width_;
    outputDescription.Height = height_;
    outputDescription.MipLevels = 1;
    outputDescription.ArraySize = 1;
    outputDescription.Format = DXGI_FORMAT_NV12;
    outputDescription.SampleDesc = { 1, 0 };
    outputDescription.Usage = D3D11_USAGE_DEFAULT;
    outputDescription.BindFlags = D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE;
    ComPtr<ID3D11Device> device;
    source->GetDevice(&device);
    ComPtr<ID3D11Texture2D> output;
    check_hresult(device->CreateTexture2D(&outputDescription, nullptr, &output));

    D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC inputDescription{};
    inputDescription.FourCC = 0;
    inputDescription.ViewDimension = D3D11_VPIV_DIMENSION_TEXTURE2D;
    inputDescription.Texture2D.MipSlice = 0;
    inputDescription.Texture2D.ArraySlice = 0;
    ComPtr<ID3D11VideoProcessorInputView> inputView;
    check_hresult(videoDevice_->CreateVideoProcessorInputView(
      source,
      enumerator_.Get(),
      &inputDescription,
      &inputView
    ));

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
  ComPtr<ID3D11VideoDevice> videoDevice_;
  ComPtr<ID3D11VideoContext> videoContext_;
  ComPtr<ID3D11VideoProcessorEnumerator> enumerator_;
  ComPtr<ID3D11VideoProcessor> processor_;
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

struct MediaSignature {
  GUID subtype{};
  UINT32 width = 0;
  UINT32 height = 0;
  UINT32 frameRateNumerator = 0;
  UINT32 frameRateDenominator = 0;
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

bool remuxChunks(
  const std::vector<fs::path>& inputs,
  const fs::path& output,
  LONGLONG requestedStart = 0,
  LONGLONG requestedDuration = LLONG_MAX
) {
  if (inputs.empty()) return false;
  const LONGLONG effectiveStart = findCleanRangeStart(inputs, requestedStart);
  const LONGLONG requestedEnd = requestedDuration == LLONG_MAX
    ? LLONG_MAX
    : requestedStart + requestedDuration;
  ComPtr<IMFSinkWriter> sink;
  DWORD sinkStream = 0;
  std::optional<DWORD> sinkAudioStream;
  LONGLONG outputTime = 0;
  std::optional<MediaSignature> expectedSignature;

  for (std::size_t fileIndex = 0; fileIndex < inputs.size(); ++fileIndex) {
    auto reader = createCompressedVideoReader(inputs[fileIndex]);

    ComPtr<IMFMediaType> mediaType;
    check_hresult(reader->GetCurrentMediaType(
      static_cast<DWORD>(MF_SOURCE_READER_FIRST_VIDEO_STREAM),
      &mediaType
    ));
    const auto signature = mediaSignature(inputs[fileIndex]);
    if (expectedSignature && !sameMediaSignature(*expectedSignature, signature)) {
      throw std::runtime_error("해상도 또는 인코딩 형식이 다른 청크는 결합할 수 없습니다");
    }
    expectedSignature = signature;
    const LONGLONG defaultDuration = fallbackSampleDuration(signature);
    auto audioReader = createCompressedAudioReader(inputs[fileIndex]);
    ComPtr<IMFMediaType> audioType;
    if (audioReader) {
      check_hresult(audioReader->GetCurrentMediaType(
        static_cast<DWORD>(MF_SOURCE_READER_FIRST_AUDIO_STREAM),
        &audioType
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
      if (audioType) {
        DWORD audioStream = 0;
        check_hresult(sink->AddStream(audioType.Get(), &audioStream));
        check_hresult(sink->SetInputMediaType(
          audioStream,
          audioType.Get(),
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
      check_hresult(sample->SetSampleTime(globalTime - requestedStart));
      UINT64 decodeTimestamp = 0;
      if (SUCCEEDED(sample->GetUINT64(
        MFSampleExtension_DecodeTimestamp,
        &decodeTimestamp
      ))) {
        const auto signedDecodeTimestamp = static_cast<LONGLONG>(decodeTimestamp);
        const auto globalDecodeTimestamp =
          outputTime + signedDecodeTimestamp - firstTime;
        check_hresult(sample->SetUINT64(
          MFSampleExtension_DecodeTimestamp,
          static_cast<UINT64>(globalDecodeTimestamp - requestedStart)
        ));
      }
      check_hresult(sink->WriteSample(sinkStream, sample.Get()));
    }
    outputTime = fileEnd;

    if (audioReader && sinkAudioStream) {
      LONGLONG firstAudioTime = -1;
      const LONGLONG defaultAudioDuration =
        signature.audioSampleRate > 0
          ? 1024ll * 10000000ll / signature.audioSampleRate
          : 1024ll * 10000000ll / AudioSampleRate;
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
        if (firstAudioTime < 0) firstAudioTime = timestamp;
        const LONGLONG duration = defaultAudioDuration;
        const LONGLONG globalTime =
          fileStart + std::max<LONGLONG>(0, timestamp - firstAudioTime);
        if (globalTime + duration <= effectiveStart) continue;
        if (globalTime >= requestedEnd) break;
        const LONGLONG outputSampleTime = globalTime - requestedStart;
        check_hresult(sample->SetSampleTime(outputSampleTime));
        check_hresult(sample->SetSampleDuration(duration));
        UINT64 decodeTimestamp = 0;
        if (SUCCEEDED(sample->GetUINT64(
          MFSampleExtension_DecodeTimestamp,
          &decodeTimestamp
        ))) {
          check_hresult(sample->SetUINT64(
            MFSampleExtension_DecodeTimestamp,
            static_cast<UINT64>(outputSampleTime)
          ));
        }
        check_hresult(sink->WriteSample(*sinkAudioStream, sample.Get()));
      }
    }
  }
  if (!sink) return false;
  check_hresult(sink->Finalize());
  return true;
}

bool remuxChunksAtomically(
  const std::vector<fs::path>& inputs,
  const fs::path& output,
  LONGLONG requestedStart = 0,
  LONGLONG requestedDuration = LLONG_MAX
) {
  auto partial = output;
  partial += L".partial.mp4";
  std::error_code cleanupError;
  fs::remove(partial, cleanupError);
  try {
    if (!remuxChunks(inputs, partial, requestedStart, requestedDuration)) return false;
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

std::int64_t fileModifiedMilliseconds(const fs::path& path) {
  WIN32_FILE_ATTRIBUTE_DATA data{};
  if (!GetFileAttributesExW(path.c_str(), GetFileExInfoStandard, &data)) return 0;
  ULARGE_INTEGER ticks{};
  ticks.LowPart = data.ftLastWriteTime.dwLowDateTime;
  ticks.HighPart = data.ftLastWriteTime.dwHighDateTime;
  constexpr std::uint64_t WindowsToUnixEpoch100ns = 116444736000000000ull;
  if (ticks.QuadPart <= WindowsToUnixEpoch100ns) return 0;
  return static_cast<std::int64_t>(
    (ticks.QuadPart - WindowsToUnixEpoch100ns) / 10000ull
  );
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
  const auto earliest = anchorMilliseconds - static_cast<std::int64_t>(seconds + 4) * 1000;
  chunks.erase(
    std::remove_if(chunks.begin(), chunks.end(), [&](const fs::path& path) {
      const auto modified = fileModifiedMilliseconds(path);
      const auto started = chunkStartedMilliseconds(path);
      return modified < earliest || started <= 0 || started > anchorMilliseconds;
    }),
    chunks.end()
  );
  return chunks;
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
    const Options& options,
    SharedStatus& status
  ) : device_(std::move(device)),
      ringDirectory_(std::move(ringDirectory)),
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
      std::lock_guard lock(status_.mutex);
      status_.droppedFrames += discarded.size();
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
        pruneRing(
          ringDirectory_,
          static_cast<std::uint64_t>(options_.capacityGb) * Gigabyte
        );
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
          clearRingDirectory(ringDirectory_);
          std::lock_guard lock(status_.mutex);
          status_.bytesUsed = 0;
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
            std::lock_guard lock(status_.mutex);
            ++status_.droppedFrames;
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
      std::lock_guard lock(status_.mutex);
      ++status_.droppedFrames;
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
      std::lock_guard lock(status_.mutex);
      ++status_.droppedFrames;
      texturePool_->release(std::move(packet.texture));
    } else {
      if (!nextFrameAt_) {
        nextFrameAt_ = capturedAt + minimumFrameInterval_;
      } else {
        *nextFrameAt_ += minimumFrameInterval_;
        if (*nextFrameAt_ < capturedAt) {
          nextFrameAt_ = capturedAt + minimumFrameInterval_;
        }
      }
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
    snapshot.droppedFrames = status.droppedFrames;
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
    << "\"droppedFrames\":" << snapshot.droppedFrames << ","
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

int runUtilityMode(const std::map<std::wstring, std::wstring>& arguments) {
  const auto mode = arguments.at(L"mode");
  init_apartment(apartment_type::multi_threaded);
  check_hresult(MFStartup(MF_VERSION, MFSTARTUP_FULL));
  SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS);
  try {
    if (mode == L"track") {
      const fs::path ringPath = arguments.at(L"ring-path");
      const fs::path outputPath = arguments.at(L"output");
      const auto anchorMilliseconds = wideInteger(
        arguments,
        L"anchor-ms",
        epochMilliseconds()
      );
      const int seconds = std::clamp(
        static_cast<int>(wideInteger(arguments, L"seconds", 60)),
        30,
        21600
      );
      const auto chunks = compatibleChunkSuffix(selectAnchoredChunks(
          ringPath,
          anchorMilliseconds,
          seconds
      ));
      if (chunks.empty()) throw std::runtime_error("편집할 녹화 청크가 없습니다");
      fs::create_directories(outputPath.parent_path());
      const auto totalDuration = combinedMediaDuration(chunks);
      const auto requestedDuration = std::min<LONGLONG>(
        totalDuration,
        static_cast<LONGLONG>(seconds) * 10000000ll
      );
      const auto requestedStart = std::max<LONGLONG>(
        0,
        totalDuration - requestedDuration
      );
      if (!remuxChunksAtomically(
        chunks,
        outputPath,
        requestedStart,
        requestedDuration
      )) {
        throw std::runtime_error("편집 트랙을 만들지 못했습니다");
      }
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
  if (
    arguments.count(L"mode")
    && (arguments.at(L"mode") == L"track" || arguments.at(L"mode") == L"extract")
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
    SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS);
    init_apartment(apartment_type::multi_threaded);
    check_hresult(MFStartup(MF_VERSION, MFSTARTUP_FULL));
    fs::create_directories(options.storagePath / L"Ring");
    fs::create_directories(options.storagePath / L"Clips");
    removeIncompleteChunks(options.storagePath / L"Ring");

    ComPtr<ID3D11Device> device;
    ComPtr<ID3D11DeviceContext> context;
    const auto winrtDevice = createWinrtDevice(device, context);
    EncoderWorker encoder(device, options.storagePath / L"Ring", options, status);
    std::unique_ptr<WindowCapture> capture;
    std::unique_ptr<ProcessAudioCapture> audioCapture;
    auto lastStatusWrite = std::chrono::steady_clock::now() - 2s;

    while (status.running && processRunning(options.parentPid)) {
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
      }
      if (encoder.failed()) {
        status.running = false;
        break;
      }
      if (!capture) {
        const HWND window = findGameWindow(options.gamePath);
        if (window) {
          try {
            capture = std::make_unique<WindowCapture>(
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
            if (gameProcessId != 0) {
              audioCapture = std::make_unique<ProcessAudioCapture>(
                gameProcessId,
                encoder,
                status
              );
            }
            std::lock_guard lock(status.mutex);
            status.waitingForGame = false;
            status.error.clear();
          } catch (const hresult_error& error) {
            std::lock_guard lock(status.mutex);
            status.error = L"게임 화면 캡처 시작 실패: " + std::wstring(error.message());
          }
        }
      }

      const auto now = std::chrono::steady_clock::now();
      if (now - lastStatusWrite >= 1s) {
        {
          std::lock_guard lock(status.mutex);
          status.bytesUsed = directoryBytes(options.storagePath / L"Ring");
          const double estimatedCompletedSeconds =
            static_cast<double>(status.bytesUsed) * 8.0
            / (
              static_cast<double>(options.bitrateMbps) * 1000000.0
              + 192000.0
            );
          status.durationSeconds =
            estimatedCompletedSeconds + encoder.currentChunkDurationSeconds();
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
