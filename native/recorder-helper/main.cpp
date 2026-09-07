#include <windows.h>
#include <comdef.h>
#include <d3d10_1.h>
#include <d3d11.h>
#include <dxgi1_2.h>
#include <mfapi.h>
#include <mfidl.h>
#include <mfreadwrite.h>
#include <shlwapi.h>
#include <wrl.h>
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
#include <filesystem>
#include <fstream>
#include <functional>
#include <iomanip>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <queue>
#include <regex>
#include <sstream>
#include <string>
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

struct Options {
  fs::path statusPath;
  fs::path controlPath;
  fs::path storagePath;
  fs::path gamePath;
  std::wstring codec = L"h264";
  int fps = 60;
  int bitrateMbps = 12;
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
  std::wstring codec = L"h264";
  std::wstring error;
  std::wstring latestClip;
  std::uint64_t bytesUsed = 0;
  std::uint64_t capacityBytes = 0;
  std::uint64_t droppedFrames = 0;
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

std::int64_t epochMilliseconds() {
  return std::chrono::duration_cast<std::chrono::milliseconds>(
    std::chrono::system_clock::now().time_since_epoch()
  ).count();
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
    if (name.rfind(L"chunk-", 0) == 0 && entry.path().extension() == L".mp4") {
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
  Nv12Converter(ID3D11Device* device, int width, int height, int fps)
    : width_(0),
      height_(0) {
    const double scale = std::min(
      1.0,
      std::min(1920.0 / static_cast<double>(width), 1080.0 / static_cast<double>(height))
    );
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
    const std::wstring& codec
  ) : path_(path), fps_(fps), converter_(device, width, height, fps) {
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

  void finalize() {
    if (!writer_) return;
    const HRESULT result = writer_->Finalize();
    writer_.Reset();
    if (FAILED(result)) {
      std::error_code error;
      fs::remove(path_, error);
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
  ComPtr<IMFSinkWriter> writer_;
};

bool remuxChunks(const std::vector<fs::path>& inputs, const fs::path& output) {
  if (inputs.empty()) return false;
  ComPtr<IMFSinkWriter> sink;
  DWORD sinkStream = 0;
  LONGLONG outputTime = 0;

  for (std::size_t fileIndex = 0; fileIndex < inputs.size(); ++fileIndex) {
    ComPtr<IMFSourceReader> reader;
    check_hresult(MFCreateSourceReaderFromURL(inputs[fileIndex].c_str(), nullptr, &reader));
    check_hresult(reader->SetStreamSelection(
      static_cast<DWORD>(MF_SOURCE_READER_ALL_STREAMS),
      FALSE
    ));
    check_hresult(reader->SetStreamSelection(
      static_cast<DWORD>(MF_SOURCE_READER_FIRST_VIDEO_STREAM),
      TRUE
    ));

    ComPtr<IMFMediaType> mediaType;
    check_hresult(reader->GetCurrentMediaType(
      static_cast<DWORD>(MF_SOURCE_READER_FIRST_VIDEO_STREAM),
      &mediaType
    ));
    if (!sink) {
      check_hresult(MFCreateSinkWriterFromURL(output.c_str(), nullptr, nullptr, &sink));
      check_hresult(sink->AddStream(mediaType.Get(), &sinkStream));
      check_hresult(sink->SetInputMediaType(sinkStream, mediaType.Get(), nullptr));
      check_hresult(sink->BeginWriting());
    }

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
      if (flags & MF_SOURCE_READERF_ENDOFSTREAM) break;
      if (!sample) continue;
      if (firstTime < 0) firstTime = timestamp;
      LONGLONG duration = 0;
      if (FAILED(sample->GetSampleDuration(&duration)) || duration <= 0) {
        duration = 10000000ll / 60;
      }
      const LONGLONG adjusted = outputTime + std::max<LONGLONG>(0, timestamp - firstTime);
      check_hresult(sample->SetSampleTime(adjusted));
      check_hresult(sink->WriteSample(sinkStream, sample.Get()));
      fileEnd = std::max(fileEnd, adjusted + duration);
    }
    outputTime = fileEnd;
  }
  if (!sink) return false;
  check_hresult(sink->Finalize());
  return true;
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
      const auto sourceFiles = selectRecentChunks(ringDirectory, seconds);
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
      if (!remuxChunks(protectedFiles, output)) {
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

  void flush() {
    std::lock_guard lock(mutex_);
    flushRequested_ = true;
    condition_.notify_one();
  }

  void requestClip(int seconds) {
    std::lock_guard lock(mutex_);
    clipSeconds_ = seconds;
    flushRequested_ = true;
    condition_.notify_one();
  }

  bool failed() const {
    return failed_;
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
  void closeWriter() {
    if (!writer_) return;
    const auto path = writer_->path();
    writer_->finalize();
    writer_.reset();
    std::error_code error;
    if (!fs::exists(path, error) || fs::file_size(path, error) == 0) {
      fs::remove(path, error);
    } else {
      pruneRing(ringDirectory_, static_cast<std::uint64_t>(options_.capacityGb) * Gigabyte);
    }
    chunkStartedAt_.reset();
    writerWidth_ = 0;
    writerHeight_ = 0;
  }

  void startWriter(const FramePacket& packet) {
    fs::create_directories(ringDirectory_);
    const auto output = ringDirectory_
      / (L"chunk-" + std::to_wstring(epochMilliseconds()) + L".mp4");
    writer_ = std::make_unique<Mp4Writer>(
      device_.Get(),
      output,
      packet.width,
      packet.height,
      options_.fps,
      options_.bitrateMbps,
      options_.codec
    );
    chunkStartedAt_ = packet.capturedAt;
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
      bool hasPacket = false;
      bool flush = false;
      int clipSeconds = 0;
      {
        std::unique_lock lock(mutex_);
        condition_.wait(lock, [&] {
          return stopping_ || !queue_.empty() || flushRequested_;
        });
        if (!queue_.empty()) {
          packet = std::move(queue_.front());
          queue_.pop();
          hasPacket = true;
        }
        flush = flushRequested_;
        flushRequested_ = false;
        clipSeconds = clipSeconds_;
        clipSeconds_ = 0;
        if (stopping_ && !hasPacket) {
          closeWriter();
          break;
        }
      }
      try {
        if (flush) closeWriter();
        if (clipSeconds > 0) {
          if (clipThread_.joinable()) clipThread_.join();
          clipThread_ = std::thread([this, clipSeconds] {
            createClip(ringDirectory_, clipSeconds, status_);
          });
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
        if (!writer_) startWriter(packet);
        const auto elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
          packet.capturedAt - *chunkStartedAt_
        ).count() / 100;
        writer_->write(packet.texture.Get(), std::max<LONGLONG>(0, elapsed));
      } catch (const hresult_error& error) {
        closeWriter();
        failed_ = true;
        std::lock_guard lock(status_.mutex);
        status_.recording = false;
        status_.error = L"녹화 인코더 오류: " + std::wstring(error.message());
      } catch (const std::exception&) {
        closeWriter();
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
  bool stopping_ = false;
  std::atomic_bool failed_ = false;
  bool flushRequested_ = false;
  int clipSeconds_ = 0;
  std::thread thread_;
  std::thread clipThread_;
  std::unique_ptr<Mp4Writer> writer_;
  std::optional<std::chrono::steady_clock::time_point> chunkStartedAt_;
  int writerWidth_ = 0;
  int writerHeight_ = 0;
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
    const auto capturedAt = std::chrono::steady_clock::now();
    if (
      nextFrameAt_
      && capturedAt + 1ms < *nextFrameAt_
    ) return;
    auto frame = sender.TryGetNextFrame();
    if (!frame) return;
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
    snapshot.codec = status.codec;
    snapshot.error = status.error;
    snapshot.latestClip = status.latestClip;
    snapshot.bytesUsed = status.bytesUsed;
    snapshot.capacityBytes = status.capacityBytes;
    snapshot.droppedFrames = status.droppedFrames;
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
    << "\"codec\":\"" << jsonEscape(snapshot.codec) << "\","
    << "\"error\":" << (snapshot.error.empty()
      ? "null"
      : "\"" + jsonEscape(snapshot.error) + "\"") << ","
    << "\"latestClip\":" << (snapshot.latestClip.empty()
      ? "null"
      : "\"" + jsonEscape(snapshot.latestClip) + "\"") << ","
    << "\"bytesUsed\":" << snapshot.bytesUsed << ","
    << "\"capacityBytes\":" << snapshot.capacityBytes << ","
    << "\"droppedFrames\":" << snapshot.droppedFrames << ","
    << "\"width\":" << snapshot.width << ","
    << "\"height\":" << snapshot.height << ","
    << "\"fps\":" << snapshot.fps << ","
    << "\"pid\":" << GetCurrentProcessId() << ","
    << "\"updatedAt\":" << epochMilliseconds()
    << "}";
  writeTextAtomic(options.statusPath, output.str());
}

std::optional<std::pair<std::string, int>> readControl(const fs::path& path) {
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
  return std::pair{ commandMatch[1].str(), seconds };
}

} 

int wmain(int count, wchar_t** values) {
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

    ComPtr<ID3D11Device> device;
    ComPtr<ID3D11DeviceContext> context;
    const auto winrtDevice = createWinrtDevice(device, context);
    EncoderWorker encoder(device, options.storagePath / L"Ring", options, status);
    std::unique_ptr<WindowCapture> capture;
    auto lastStatusWrite = std::chrono::steady_clock::now() - 2s;

    while (status.running && processRunning(options.parentPid)) {
      if (const auto control = readControl(options.controlPath)) {
        if (control->first == "stop") {
          status.running = false;
          break;
        }
        if (control->first == "clip") encoder.requestClip(control->second);
      }

      if (capture && capture->closed()) {
        capture.reset();
        std::lock_guard lock(status.mutex);
        status.recording = false;
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
        }
        try {
          updateStatusFile(options, status);
        } catch (...) {
        }
        lastStatusWrite = now;
      }
      std::this_thread::sleep_for(100ms);
    }

    capture.reset();
    encoder.stop();
    {
      std::lock_guard lock(status.mutex);
      status.running = false;
      status.recording = false;
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
