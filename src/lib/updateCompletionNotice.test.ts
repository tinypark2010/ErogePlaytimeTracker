import { describe, expect, it } from 'vitest';
import { render } from 'svelte/server';
import UpdateCompletionNotice from '../components/UpdateCompletionNotice.svelte';
import type { UpdateCompletionNotice as Notice } from './types';

function markup(notice: Notice) {
  return render(UpdateCompletionNotice, { props: { notice, onclose: () => {} } }).body;
}

describe('update completion notification content', () => {
  it('confirms the installed version even with no user-facing changes', () => {
    const html = markup({ version: '1.2.3', releases: [] });
    expect(html).toContain('アップデートが完了しました');
    expect(html).toContain('バージョン 1.2.3 に更新しました。');
    expect(html).not.toContain('<ul>');
    expect(html).not.toContain('<h3>更新内容</h3>');
  });

  it('renders the supplied Japanese changes as escaped text', () => {
    const html = markup({
      version: '1.2.3',
      releases: [
        { version: '1.2.3', changes: ['検索機能を追加しました。', '<script>表示例</script>'] },
      ],
    });
    expect(html).toContain('<h3>更新内容</h3>');
    expect(html).toContain('<li>検索機能を追加しました。</li>');
    expect(html).toContain('&lt;script>表示例&lt;/script>');
  });

  it('labels intervening releases and omits empty sections', () => {
    const html = markup({
      version: '1.2.3',
      releases: [
        { version: '1.2.3', changes: [] },
        { version: '1.2.2', changes: ['機能を追加しました。'] },
        { version: '1.2.1', changes: ['不具合を修正しました。'] },
      ],
    });
    expect(html).toContain('<h4>バージョン 1.2.2</h4>');
    expect(html).toContain('<h4>バージョン 1.2.1</h4>');
    expect(html).not.toContain('<h4>バージョン 1.2.3</h4>');
    expect(html.indexOf('機能を追加しました。')).toBeLessThan(
      html.indexOf('不具合を修正しました。'),
    );
  });
});
