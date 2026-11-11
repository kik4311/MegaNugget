#pragma once

#include <QString>

struct Compatibility {
  enum class Status { ResearchTarget, Unverified };

  Status status = Status::Unverified;
  bool jailbreakAvailable = false;
  QString reason;
  bool matchesTarget = false;
};

Compatibility evaluateCompatibility(const QString& product, const QString& ios);
