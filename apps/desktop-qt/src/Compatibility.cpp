#include "Compatibility.h"

Compatibility evaluateCompatibility(const QString& product, const QString& ios) {
  const bool target =
      (product == QLatin1String("iPhone9,1") || product == QLatin1String("iPhone9,3")) &&
      ios == QLatin1String("15.8.8");

  Compatibility result;
  result.matchesTarget = target;
  result.jailbreakAvailable = false;
  if (target) {
    result.status = Compatibility::Status::ResearchTarget;
    result.reason = QStringLiteral(
        "Target hardware/version; no verified jailbreak method.");
  } else {
    result.status = Compatibility::Status::Unverified;
    result.reason = QStringLiteral(
        "This hardware/version combination has not been verified.");
  }
  return result;
}
