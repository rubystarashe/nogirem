export function assessExitConfirmation({
  hasLiveAppliedAffinity,
  hasManagedNic,
}) {
  return {
    confirm: Boolean(hasLiveAppliedAffinity),
    removeAppliedMarker: !hasLiveAppliedAffinity && !hasManagedNic,
  }
}
