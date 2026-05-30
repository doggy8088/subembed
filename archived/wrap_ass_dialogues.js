#!/usr/bin/env node
'use strict';

const fs = require('node:fs');

const OPENING_PUNCTUATION = new Set('([{<（［｛〈《「『【〔“‘');
const PREFERRED_BREAK_PUNCTUATION = new Set('，。,.!?！？；、');
const FORCE_BREAK_PUNCTUATION = new Set(',.;:!?)]}/-–—，。．、；：！？）》」』】〕）');
const REQUIRED_FLAGS = [
  '--input',
  '--output',
  '--video-width',
  '--font-size',
  '--margin-left',
  '--margin-right',
  '--outline-size',
  '--shadow-size',
];

const MARK_RE = /\p{M}/u;
const FORMAT_RE = /\p{Cf}/u;
const DIGIT_RE = /\p{Nd}/u;
const PUNCTUATION_RE = /\p{P}/u;
const LETTER_RE = /\p{L}/u;
const LETTER_OR_NUMBER_RE = /[\p{L}\p{N}]/u;
const LATIN_RE = /\p{Script=Latin}/u;
const WHITESPACE_RE = /\s/u;

const FULLWIDTH_RANGES = [
  [0xff01, 0xff60],
  [0xffe0, 0xffe6],
];

const WIDE_RANGES = [
  [0x1100, 0x115f],
  [0x231a, 0x231b],
  [0x2329, 0x232a],
  [0x23e9, 0x23ec],
  [0x23f0, 0x23f0],
  [0x23f3, 0x23f3],
  [0x25fd, 0x25fe],
  [0x2614, 0x2615],
  [0x2648, 0x2653],
  [0x267f, 0x267f],
  [0x2693, 0x2693],
  [0x26a1, 0x26a1],
  [0x26aa, 0x26ab],
  [0x26bd, 0x26be],
  [0x26c4, 0x26c5],
  [0x26ce, 0x26ce],
  [0x26d4, 0x26d4],
  [0x26ea, 0x26ea],
  [0x26f2, 0x26f3],
  [0x26f5, 0x26f5],
  [0x26fa, 0x26fa],
  [0x26fd, 0x26fd],
  [0x2705, 0x2705],
  [0x270a, 0x270b],
  [0x2728, 0x2728],
  [0x274c, 0x274c],
  [0x274e, 0x274e],
  [0x2753, 0x2755],
  [0x2757, 0x2757],
  [0x2795, 0x2797],
  [0x27b0, 0x27b0],
  [0x27bf, 0x27bf],
  [0x2b1b, 0x2b1c],
  [0x2b50, 0x2b50],
  [0x2b55, 0x2b55],
  [0x2e80, 0x2ffb],
  [0x3000, 0x303e],
  [0x3041, 0x33ff],
  [0x3400, 0x4dbf],
  [0x4e00, 0xa4c6],
  [0xa960, 0xa97c],
  [0xac00, 0xd7a3],
  [0xf900, 0xfaff],
  [0xfe10, 0xfe19],
  [0xfe30, 0xfe6b],
  [0x1f004, 0x1f004],
  [0x1f0cf, 0x1f0cf],
  [0x1f18e, 0x1f18e],
  [0x1f191, 0x1f19a],
  [0x1f200, 0x1f251],
  [0x1f300, 0x1f64f],
  [0x1f680, 0x1f6ff],
  [0x1f900, 0x1f9ff],
  [0x20000, 0x2fffd],
  [0x30000, 0x3fffd],
];

const AMBIGUOUS_RANGES = [
  [0x00a1, 0x00a1], [0x00a4, 0x00a4], [0x00a7, 0x00a8], [0x00aa, 0x00aa], [0x00ad, 0x00ae],
  [0x00b0, 0x00b4], [0x00b6, 0x00ba], [0x00bc, 0x00bf], [0x00c6, 0x00c6], [0x00d0, 0x00d0],
  [0x00d7, 0x00d8], [0x00de, 0x00e1], [0x00e6, 0x00e6], [0x00e8, 0x00ea], [0x00ec, 0x00ed],
  [0x00f0, 0x00f0], [0x00f2, 0x00f3], [0x00f7, 0x00fa], [0x00fc, 0x00fc], [0x00fe, 0x00fe],
  [0x0101, 0x0101], [0x0111, 0x0111], [0x0113, 0x0113], [0x011b, 0x011b], [0x0126, 0x0127],
  [0x012b, 0x012b], [0x0131, 0x0133], [0x0138, 0x0138], [0x013f, 0x0142], [0x0144, 0x0144],
  [0x0148, 0x014b], [0x014d, 0x014d], [0x0152, 0x0153], [0x0166, 0x0167], [0x016b, 0x016b],
  [0x01ce, 0x01ce], [0x01d0, 0x01d0], [0x01d2, 0x01d2], [0x01d4, 0x01d4], [0x01d6, 0x01d6],
  [0x01d8, 0x01d8], [0x01da, 0x01da], [0x01dc, 0x01dc], [0x0251, 0x0251], [0x0261, 0x0261],
  [0x02c4, 0x02c4], [0x02c7, 0x02c7], [0x02c9, 0x02cb], [0x02cd, 0x02cd], [0x02d0, 0x02d0],
  [0x02d8, 0x02db], [0x02dd, 0x02dd], [0x02df, 0x02df], [0x0300, 0x036f], [0x0391, 0x03a1],
  [0x03a3, 0x03a9], [0x03b1, 0x03c1], [0x03c3, 0x03c9], [0x0401, 0x0401], [0x0410, 0x044f],
  [0x0451, 0x0451], [0x2010, 0x2010], [0x2013, 0x2016], [0x2018, 0x2019], [0x201c, 0x201d],
  [0x2020, 0x2022], [0x2024, 0x2027], [0x2030, 0x2030], [0x2032, 0x2033], [0x2035, 0x2035],
  [0x203b, 0x203b], [0x203e, 0x203e], [0x2074, 0x2074], [0x207f, 0x207f], [0x2081, 0x2084],
  [0x20ac, 0x20ac], [0x2103, 0x2103], [0x2105, 0x2105], [0x2109, 0x2109], [0x2113, 0x2113],
  [0x2116, 0x2116], [0x2121, 0x2122], [0x2126, 0x2126], [0x212b, 0x212b], [0x2153, 0x2154],
  [0x215b, 0x215e], [0x2160, 0x216b], [0x2170, 0x2179], [0x2189, 0x2189], [0x2190, 0x2199],
  [0x21b8, 0x21b9], [0x21d2, 0x21d2], [0x21d4, 0x21d4], [0x21e7, 0x21e7], [0x2200, 0x2200],
  [0x2202, 0x2203], [0x2207, 0x2208], [0x220b, 0x220b], [0x220f, 0x220f], [0x2211, 0x2211],
  [0x2215, 0x2215], [0x221a, 0x221a], [0x221d, 0x2220], [0x2223, 0x2223], [0x2225, 0x2225],
  [0x2227, 0x222c], [0x222e, 0x222e], [0x2234, 0x2237], [0x223c, 0x223d], [0x2248, 0x2248],
  [0x224c, 0x224c], [0x2252, 0x2252], [0x2260, 0x2261], [0x2264, 0x2267], [0x226a, 0x226b],
  [0x226e, 0x226f], [0x2282, 0x2283], [0x2286, 0x2287], [0x2295, 0x2295], [0x2299, 0x2299],
  [0x22a5, 0x22a5], [0x22bf, 0x22bf], [0x2312, 0x2312], [0x2460, 0x24e9], [0x24eb, 0x254b],
  [0x2550, 0x2573], [0x2580, 0x258f], [0x2592, 0x2595], [0x25a0, 0x25a1], [0x25a3, 0x25a9],
  [0x25b2, 0x25b3], [0x25b6, 0x25b7], [0x25bc, 0x25bd], [0x25c0, 0x25c1], [0x25c6, 0x25c8],
  [0x25cb, 0x25cb], [0x25ce, 0x25d1], [0x25e2, 0x25e5], [0x25ef, 0x25ef], [0x2605, 0x2606],
  [0x2609, 0x2609], [0x260e, 0x260f], [0x261c, 0x261c], [0x261e, 0x261e], [0x2640, 0x2640],
  [0x2642, 0x2642], [0x2660, 0x2661], [0x2663, 0x2665], [0x2667, 0x266a], [0x266c, 0x266d],
  [0x266f, 0x266f], [0x269e, 0x269f], [0x26bf, 0x26bf], [0x26c6, 0x26cd], [0x26cf, 0x26d3],
  [0x26d5, 0x26e1], [0x26e3, 0x26e3], [0x26e8, 0x26e9], [0x26eb, 0x26f1], [0x26f4, 0x26f4],
  [0x26f6, 0x26f9], [0x26fb, 0x26fc], [0x26fe, 0x26ff], [0x273d, 0x273d], [0x2776, 0x277f],
  [0xe000, 0xf8ff], [0xfe00, 0xfe0f], [0xfffd, 0xfffd], [0x1f100, 0x1f10a], [0x1f110, 0x1f12d],
  [0x1f130, 0x1f169], [0x1f170, 0x1f18d], [0x1f18f, 0x1f190], [0x1f19b, 0x1f1ac],
  [0xe0100, 0xe01ef], [0xf0000, 0xffffd], [0x100000, 0x10fffd],
];

function usage() {
  return [
    '用法：',
    '  wrap_ass_dialogues.py --input INPUT --output OUTPUT --video-width N --font-size N --margin-left N --margin-right N --outline-size N --shadow-size N',
  ].join('\n');
}

function inRanges(codePoint, ranges) {
  for (const [start, end] of ranges) {
    if (codePoint >= start && codePoint <= end) {
      return true;
    }
  }
  return false;
}

function eastAsianWidth(char) {
  const codePoint = char.codePointAt(0);
  if (codePoint === undefined) {
    return 'N';
  }
  if (inRanges(codePoint, FULLWIDTH_RANGES)) {
    return 'F';
  }
  if (inRanges(codePoint, WIDE_RANGES)) {
    return 'W';
  }
  if ((codePoint >= 0x20 && codePoint <= 0x7e) || (codePoint >= 0xff61 && codePoint <= 0xff9f)) {
    return 'Na';
  }
  if (inRanges(codePoint, AMBIGUOUS_RANGES)) {
    return 'A';
  }
  return 'N';
}

function isUppercaseLetter(char) {
  return LETTER_RE.test(char) && char.toUpperCase() === char && char.toLowerCase() !== char;
}

function estimateCharWidth(char) {
  if (!char) {
    return 0;
  }
  if (MARK_RE.test(char) || FORMAT_RE.test(char)) {
    return 0;
  }
  if (char === ' ' || char === '\t') {
    return 0.35;
  }
  if (char.codePointAt(0) >= 0x1f300) {
    return 2.0;
  }

  const widthClass = eastAsianWidth(char);
  if (widthClass === 'F' || widthClass === 'W') {
    return 1.0;
  }
  if (DIGIT_RE.test(char)) {
    return 0.56;
  }
  if (PUNCTUATION_RE.test(char)) {
    return widthClass === 'Na' ? 0.45 : 0.6;
  }
  if (LATIN_RE.test(char)) {
    return isUppercaseLetter(char) ? 0.62 : 0.52;
  }
  if (widthClass === 'A') {
    return 0.8;
  }
  if (LETTER_RE.test(char)) {
    return 0.68;
  }
  return 0.6;
}

function isPreferredBreakChar(char) {
  return Boolean(char) && !OPENING_PUNCTUATION.has(char) && PREFERRED_BREAK_PUNCTUATION.has(char);
}

function isForceBreakChar(char) {
  return Boolean(char) && !OPENING_PUNCTUATION.has(char) && FORCE_BREAK_PUNCTUATION.has(char);
}

function isCjkForceBreakChar(char) {
  return (
    Boolean(char) &&
    !OPENING_PUNCTUATION.has(char) &&
    LETTER_OR_NUMBER_RE.test(char) &&
    ['F', 'W'].includes(eastAsianWidth(char))
  );
}

function readCodePoint(text, index) {
  const codePoint = text.codePointAt(index);
  const char = String.fromCodePoint(codePoint);
  return { char, nextIndex: index + char.length };
}

function tokenizeAssText(text) {
  const tokens = [];
  let index = 0;

  while (index < text.length) {
    const char = text[index];

    if (char === '{') {
      const tagEnd = text.indexOf('}', index + 1);
      if (tagEnd !== -1) {
        tokens.push({ kind: 'tag', raw: text.slice(index, tagEnd + 1), width: 0, visible: false, breakChar: '' });
        index = tagEnd + 1;
        continue;
      }
    }

    if (text.startsWith('\\N', index) || text.startsWith('\\n', index)) {
      tokens.push({ kind: 'newline', raw: text.slice(index, index + 2), width: 0, visible: false, breakChar: '' });
      index += 2;
      continue;
    }

    if (text.startsWith('\\h', index)) {
      tokens.push({ kind: 'hard-space', raw: '\\h', width: 0.35, visible: false, breakChar: '' });
      index += 2;
      continue;
    }

    if (char === '\\' && index + 1 < text.length) {
      const { char: literal, nextIndex } = readCodePoint(text, index + 1);
      tokens.push({
        kind: 'text',
        raw: `\\${literal}`,
        width: estimateCharWidth(literal),
        visible: !WHITESPACE_RE.test(literal),
        breakChar: literal,
      });
      index = nextIndex;
      continue;
    }

    const { char: value, nextIndex } = readCodePoint(text, index);
    const kind = WHITESPACE_RE.test(value) ? 'space' : 'text';
    tokens.push({
      kind,
      raw: value,
      width: estimateCharWidth(value),
      visible: kind === 'text',
      breakChar: kind === 'text' ? value : '',
    });
    index = nextIndex;
  }

  return tokens;
}

function hasVisibleText(tokens, start, end) {
  for (let index = start; index < end; index += 1) {
    if (tokens[index].visible) {
      return true;
    }
  }
  return false;
}

function isTrimmableSpace(token) {
  return token.kind === 'space';
}

function skipLeadingSpaces(tokens, start) {
  while (start < tokens.length && isTrimmableSpace(tokens[start])) {
    start += 1;
  }
  return start;
}

function keepPunctuationWithPrevious(tokens, splitAt, nextStart) {
  nextStart = skipLeadingSpaces(tokens, nextStart);

  while (
    nextStart < tokens.length &&
    tokens[nextStart].kind === 'text' &&
    isForceBreakChar(tokens[nextStart].breakChar)
  ) {
    splitAt = nextStart + 1;
    nextStart = skipLeadingSpaces(tokens, splitAt);
  }

  return [splitAt, nextStart];
}

function wrapSegment(tokens, maxUnits) {
  if (tokens.length === 0 || maxUnits <= 0) {
    return tokens.map((token) => token.raw).join('');
  }

  const wrapped = [];
  let start = 0;

  while (start < tokens.length) {
    let width = 0;
    let preferredBreak = null;
    let forceBreak = null;
    let index = start;

    while (index < tokens.length) {
      const token = tokens[index];
      width += token.width;

      if (token.kind === 'space') {
        if (hasVisibleText(tokens, start, index)) {
          forceBreak = [index, index + 1];
        }
      } else if (token.kind === 'text') {
        const char = token.breakChar;
        if (isPreferredBreakChar(char)) {
          preferredBreak = [index + 1, index + 1];
          forceBreak = [index + 1, index + 1];
        } else if (isForceBreakChar(char) || isCjkForceBreakChar(char)) {
          forceBreak = [index + 1, index + 1];
        }
      }

      if (width > maxUnits && hasVisibleText(tokens, start, index + 1)) {
        let splitAt;
        let nextStart;

        if (preferredBreak && preferredBreak[0] > start) {
          [splitAt, nextStart] = preferredBreak;
        } else if (forceBreak && forceBreak[0] > start) {
          [splitAt, nextStart] = forceBreak;
        } else {
          splitAt = hasVisibleText(tokens, start, index) ? index : index + 1;
          if (splitAt <= start) {
            splitAt = index + 1;
          }
          nextStart = splitAt;
        }

        [splitAt, nextStart] = keepPunctuationWithPrevious(tokens, splitAt, nextStart);
        wrapped.push(...tokens.slice(start, splitAt).map((current) => current.raw), '\\N');
        start = nextStart;
        break;
      }

      index += 1;
    }

    if (index >= tokens.length) {
      wrapped.push(...tokens.slice(start).map((token) => token.raw));
      break;
    }
  }

  return wrapped.join('');
}

function wrapAssText(text, maxUnits) {
  const tokens = tokenizeAssText(text);
  const wrappedParts = [];
  let currentSegment = [];

  for (const token of tokens) {
    if (token.kind === 'newline') {
      wrappedParts.push(wrapSegment(currentSegment, maxUnits), token.raw);
      currentSegment = [];
      continue;
    }
    currentSegment.push(token);
  }

  wrappedParts.push(wrapSegment(currentSegment, maxUnits));
  return wrappedParts.join('');
}

function parseIntField(value, fallback) {
  const stripped = value.trim();
  if (/^\d+$/.test(stripped)) {
    const parsed = Number.parseInt(stripped, 10);
    if (parsed > 0) {
      return parsed;
    }
  }
  return fallback;
}

function computeMaxUnits(videoWidth, marginLeft, marginRight, fontSize, outlineSize, shadowSize) {
  const sidePadding = Math.max(fontSize * 0.75, (outlineSize + shadowSize) * 6.0, 24.0);
  let usableWidth = videoWidth - marginLeft - marginRight - sidePadding * 2.0;
  usableWidth = Math.max(usableWidth, videoWidth * 0.45);
  const fullwidthPixels = Math.max(fontSize * 0.95, 1.0);
  return Math.max(8.0, Math.min(60.0, usableWidth / fullwidthPixels));
}

function splitDialogueFields(dialogueBody, textIndex) {
  const fields = [];
  let start = 0;
  let splits = 0;

  for (let index = 0; index < dialogueBody.length; index += 1) {
    if (dialogueBody[index] === ',' && splits < textIndex) {
      fields.push(dialogueBody.slice(start, index));
      start = index + 1;
      splits += 1;
    }
  }

  fields.push(dialogueBody.slice(start));
  return fields;
}

function splitOnce(text, separator) {
  const index = text.indexOf(separator);
  if (index === -1) {
    return [text, ''];
  }
  return [text.slice(0, index), text.slice(index + separator.length)];
}

function readAssLines(inputPath) {
  let content = fs.readFileSync(inputPath, 'utf8');
  content = content.replace(/\r\n?/g, '\n');
  if (content === '') {
    return [];
  }
  if (content.endsWith('\n')) {
    content = content.slice(0, -1);
  }
  return content.split('\n');
}

function wrapAssFile(inputPath, outputPath, videoWidth, defaultMarginLeft, defaultMarginRight, fontSize, outlineSize, shadowSize) {
  const lines = readAssLines(inputPath);
  let inEvents = false;
  let textIndex = null;
  let marginLeftIndex = null;
  let marginRightIndex = null;
  const outputLines = [];

  for (let line of lines) {
    if (line === '[Events]') {
      inEvents = true;
      outputLines.push(line);
      continue;
    }

    if (inEvents && line.startsWith('Format:')) {
      const [, formatBody] = splitOnce(line, ':');
      const formatFields = formatBody.split(',').map((field) => field.trim());
      textIndex = formatFields.indexOf('Text');
      if (textIndex === -1) {
        throw new Error('ASS [Events] 缺少 Text 欄位');
      }
      marginLeftIndex = formatFields.indexOf('MarginL');
      marginRightIndex = formatFields.indexOf('MarginR');
      if (marginLeftIndex === -1) {
        marginLeftIndex = null;
      }
      if (marginRightIndex === -1) {
        marginRightIndex = null;
      }
      outputLines.push(line);
      continue;
    }

    if (inEvents && line.startsWith('Dialogue:') && textIndex !== null) {
      const [, dialogueBody] = splitOnce(line, ':');
      const fields = splitDialogueFields(dialogueBody, textIndex);

      if (fields.length > textIndex) {
        let marginLeft = defaultMarginLeft;
        let marginRight = defaultMarginRight;

        if (marginLeftIndex !== null && marginLeftIndex < fields.length) {
          marginLeft = parseIntField(fields[marginLeftIndex], defaultMarginLeft);
        }
        if (marginRightIndex !== null && marginRightIndex < fields.length) {
          marginRight = parseIntField(fields[marginRightIndex], defaultMarginRight);
        }

        const maxUnits = computeMaxUnits(
          videoWidth,
          marginLeft,
          marginRight,
          fontSize,
          outlineSize,
          shadowSize,
        );
        fields[textIndex] = wrapAssText(fields[textIndex], maxUnits);
        line = `Dialogue:${fields.join(',')}`;
      }
    }

    outputLines.push(line);
  }

  fs.writeFileSync(outputPath, `${outputLines.join('\n')}\n`, 'utf8');
}

function parseArgs(argv) {
  const values = new Map();

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '-h' || arg === '--help') {
      process.stdout.write(`${usage()}\n`);
      process.exit(0);
    }

    if (!arg.startsWith('--')) {
      throw new Error(`不支援的參數：${arg}`);
    }

    const [flag, inlineValue] = arg.split(/=(.*)/s, 2);
    if (!REQUIRED_FLAGS.includes(flag)) {
      throw new Error(`不支援的參數：${flag}`);
    }

    let value = inlineValue;
    if (value === undefined) {
      index += 1;
      value = argv[index];
    }

    if (value === undefined || value === '') {
      throw new Error(`缺少參數值：${flag}`);
    }

    values.set(flag, value);
  }

  for (const flag of REQUIRED_FLAGS) {
    if (!values.has(flag)) {
      throw new Error(`缺少必要參數：${flag}`);
    }
  }

  const intFlags = ['--video-width', '--font-size', '--margin-left', '--margin-right', '--outline-size', '--shadow-size'];
  for (const flag of intFlags) {
    const value = values.get(flag);
    if (!/^\d+$/.test(value)) {
      throw new Error(`${flag} 必須是整數`);
    }
    values.set(flag, Number.parseInt(value, 10));
  }

  return {
    inputPath: values.get('--input'),
    outputPath: values.get('--output'),
    videoWidth: values.get('--video-width'),
    fontSize: values.get('--font-size'),
    marginLeft: values.get('--margin-left'),
    marginRight: values.get('--margin-right'),
    outlineSize: values.get('--outline-size'),
    shadowSize: values.get('--shadow-size'),
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  wrapAssFile(
    args.inputPath,
    args.outputPath,
    args.videoWidth,
    args.marginLeft,
    args.marginRight,
    args.fontSize,
    args.outlineSize,
    args.shadowSize,
  );
}

try {
  main();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`錯誤：${message}\n`);
  process.exit(1);
}
