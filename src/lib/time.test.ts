import {describe,expect,it} from 'vitest';
import {duration,inputTime,lastPlayed} from './time';

describe('duration',()=>{
  it('always includes seconds',()=>{
    expect(duration(0)).toBe('0秒');
    expect(duration(42)).toBe('42秒');
    expect(duration(65)).toBe('1分 5秒');
    expect(duration(7261)).toBe('2時間 1分 1秒');
  });
  it('keeps seconds in datetime-local values',()=>{
    expect(inputTime('2026-08-16T00:34:56Z')).toMatch(/:34:56$|:34:56/);
  });
  it('labels a game without play history',()=>{
    expect(lastPlayed(null)).toBe('未プレイ');
  });
});
