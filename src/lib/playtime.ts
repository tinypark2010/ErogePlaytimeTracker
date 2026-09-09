export function backgroundIntervalDescription(excludeBackgroundTime: boolean): string {
  return excludeBackgroundTime
    ? 'ゲームがバックグラウンドにあった区間です。現在の設定では、この時間をプレイ時間から除外します。'
    : 'ゲームがバックグラウンドにあった区間です。現在の設定では、この時間もプレイ時間に含めます。';
}
