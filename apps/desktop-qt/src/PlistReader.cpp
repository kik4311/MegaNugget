#include "PlistReader.h"

#include <QXmlStreamReader>

namespace {

std::optional<AppPlistInfo> parseXmlPlist(const QByteArray& bytes) {
  QXmlStreamReader xml(bytes);
  AppPlistInfo info;
  QString pendingKey;
  enum class Mode { Root, Dict, Array } mode = Mode::Root;
  QString arrayKey;
  int dictDepth = 0;

  auto assignString = [&](const QString& value) {
    if (pendingKey == QLatin1String("CFBundleIdentifier")) {
      info.bundleId = value;
    } else if (pendingKey == QLatin1String("CFBundleVersion")) {
      info.version = value;
    } else if (pendingKey == QLatin1String("CFBundleExecutable")) {
      info.executable = value;
    } else if (pendingKey == QLatin1String("CFBundlePackageType")) {
      info.packageType = value;
    } else if (pendingKey == QLatin1String("MinimumOSVersion")) {
      info.minimumIos = value;
    }
    pendingKey.clear();
  };

  while (!xml.atEnd()) {
    const auto token = xml.readNext();
    if (token == QXmlStreamReader::StartElement) {
      const auto name = xml.name();
      if (name == QLatin1String("dict")) {
        ++dictDepth;
        mode = Mode::Dict;
      } else if (name == QLatin1String("key") && mode == Mode::Dict && dictDepth == 1) {
        pendingKey = xml.readElementText();
        if (pendingKey == QLatin1String("CFBundleSupportedPlatforms")) {
          arrayKey = pendingKey;
        }
      } else if (name == QLatin1String("string")) {
        const QString value = xml.readElementText();
        if (mode == Mode::Array && arrayKey == QLatin1String("CFBundleSupportedPlatforms")) {
          info.platforms.push_back(value);
        } else if (mode == Mode::Dict && dictDepth == 1) {
          assignString(value);
        }
      } else if (name == QLatin1String("array") &&
                 pendingKey == QLatin1String("CFBundleSupportedPlatforms")) {
        mode = Mode::Array;
        pendingKey.clear();
      }
    } else if (token == QXmlStreamReader::EndElement) {
      if (xml.name() == QLatin1String("dict")) {
        --dictDepth;
        mode = dictDepth > 0 ? Mode::Dict : Mode::Root;
      } else if (xml.name() == QLatin1String("array") && mode == Mode::Array) {
        mode = Mode::Dict;
        arrayKey.clear();
      }
    }
  }
  if (xml.hasError() || info.bundleId.isEmpty()) {
    return std::nullopt;
  }
  return info;
}

quint64 readBe(const QByteArray& data, int offset, int size) {
  quint64 value = 0;
  for (int i = 0; i < size; ++i) {
    value = (value << 8) | static_cast<quint8>(data.at(offset + i));
  }
  return value;
}

// Minimal binary plist reader for top-level string / string-array values we need.
std::optional<AppPlistInfo> parseBinaryPlist(const QByteArray& data) {
  if (data.size() < 40 || !data.startsWith("bplist00")) {
    return std::nullopt;
  }
  const int trailerOffset = data.size() - 32;
  const int offsetSize = static_cast<quint8>(data.at(trailerOffset + 6));
  const int refSize = static_cast<quint8>(data.at(trailerOffset + 7));
  const quint64 numObjects = readBe(data, trailerOffset + 8, 8);
  const quint64 topObject = readBe(data, trailerOffset + 16, 8);
  const quint64 offsetTableStart = readBe(data, trailerOffset + 24, 8);
  if (offsetSize < 1 || offsetSize > 8 || refSize < 1 || refSize > 8 || numObjects == 0 ||
      topObject >= numObjects) {
    return std::nullopt;
  }

  auto objectOffset = [&](quint64 index) -> std::optional<int> {
    const qint64 pos =
        static_cast<qint64>(offsetTableStart + index * static_cast<quint64>(offsetSize));
    if (pos < 0 || pos + offsetSize > data.size()) {
      return std::nullopt;
    }
    return static_cast<int>(readBe(data, static_cast<int>(pos), offsetSize));
  };

  auto readRef = [&](int at) -> std::optional<quint64> {
    if (at + refSize > data.size()) {
      return std::nullopt;
    }
    return readBe(data, at, refSize);
  };

  auto readString = [&](quint64 index) -> std::optional<QString> {
    const auto off = objectOffset(index);
    if (!off) {
      return std::nullopt;
    }
    int cursor = 0;
    const auto marker = static_cast<quint8>(data.at(*off));
    const int type = marker >> 4;
    int info = marker & 0x0f;
    cursor = *off + 1;
    if (info == 0x0f) {
      if (cursor >= data.size()) {
        return std::nullopt;
      }
      const auto sizeMarker = static_cast<quint8>(data.at(cursor++));
      const int sizeLen = 1 << (sizeMarker & 0x0f);
      if (cursor + sizeLen > data.size()) {
        return std::nullopt;
      }
      info = static_cast<int>(readBe(data, cursor, sizeLen));
      cursor += sizeLen;
    }
    if (type == 0x5) {
      if (cursor + info > data.size()) {
        return std::nullopt;
      }
      return QString::fromLatin1(data.constData() + cursor, info);
    }
    if (type == 0x6) {
      if (cursor + info * 2 > data.size()) {
        return std::nullopt;
      }
      QString out;
      out.resize(info);
      for (int i = 0; i < info; ++i) {
        const auto ch = static_cast<char16_t>(
            (static_cast<quint8>(data.at(cursor + i * 2)) << 8) |
            static_cast<quint8>(data.at(cursor + i * 2 + 1)));
        out[i] = QChar(ch);
      }
      return out;
    }
    return std::nullopt;
  };

  const auto topOff = objectOffset(topObject);
  if (!topOff) {
    return std::nullopt;
  }
  const auto marker = static_cast<quint8>(data.at(*topOff));
  if ((marker >> 4) != 0xD) {
    return std::nullopt;
  }
  int info = marker & 0x0f;
  int cursor = *topOff + 1;
  if (info == 0x0f) {
    const auto sizeMarker = static_cast<quint8>(data.at(cursor++));
    const int sizeLen = 1 << (sizeMarker & 0x0f);
    info = static_cast<int>(readBe(data, cursor, sizeLen));
    cursor += sizeLen;
  }
  const int keyStart = cursor;
  const int valueStart = cursor + info * refSize;

  AppPlistInfo app;
  for (int i = 0; i < info; ++i) {
    const auto keyRef = readRef(keyStart + i * refSize);
    const auto valRef = readRef(valueStart + i * refSize);
    if (!keyRef || !valRef) {
      return std::nullopt;
    }
    const auto key = readString(*keyRef);
    if (!key) {
      continue;
    }
    if (*key == QLatin1String("CFBundleSupportedPlatforms")) {
      const auto arrOff = objectOffset(*valRef);
      if (!arrOff) {
        continue;
      }
      const auto arrMarker = static_cast<quint8>(data.at(*arrOff));
      if ((arrMarker >> 4) != 0xA) {
        continue;
      }
      int count = arrMarker & 0x0f;
      int arrCursor = *arrOff + 1;
      if (count == 0x0f) {
        const auto sizeMarker = static_cast<quint8>(data.at(arrCursor++));
        const int sizeLen = 1 << (sizeMarker & 0x0f);
        count = static_cast<int>(readBe(data, arrCursor, sizeLen));
        arrCursor += sizeLen;
      }
      for (int j = 0; j < count; ++j) {
        const auto itemRef = readRef(arrCursor + j * refSize);
        if (!itemRef) {
          break;
        }
        if (const auto platform = readString(*itemRef)) {
          app.platforms.push_back(*platform);
        }
      }
      continue;
    }
    if (const auto value = readString(*valRef)) {
      if (*key == QLatin1String("CFBundleIdentifier")) {
        app.bundleId = *value;
      } else if (*key == QLatin1String("CFBundleVersion")) {
        app.version = *value;
      } else if (*key == QLatin1String("CFBundleExecutable")) {
        app.executable = *value;
      } else if (*key == QLatin1String("CFBundlePackageType")) {
        app.packageType = *value;
      } else if (*key == QLatin1String("MinimumOSVersion")) {
        app.minimumIos = *value;
      }
    }
  }
  if (app.bundleId.isEmpty()) {
    return std::nullopt;
  }
  return app;
}

}  // namespace

std::optional<AppPlistInfo> parseAppPlist(const QByteArray& bytes) {
  if (bytes.startsWith("bplist")) {
    return parseBinaryPlist(bytes);
  }
  return parseXmlPlist(bytes);
}
