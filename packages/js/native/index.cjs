'use strict';

const { platform, arch } = process;
let libc = 'none';
let supportedLibc = true;

if (platform === 'linux') {
  const glibc = process.report?.getReport()?.header?.glibcVersionRuntime;
  libc = typeof glibc === 'string' ? `glibc-${glibc}` : 'musl';
  const match = typeof glibc === 'string' && /^(\d+)\.(\d+)(?:\.\d+)*$/.exec(glibc);
  supportedLibc = Boolean(match && (Number(match[1]) > 2 ||
    (Number(match[1]) === 2 && Number(match[2]) >= 28)));
}

const targets = {
  'linux/x64': 'linux-x64-gnu',
  'linux/arm64': 'linux-arm64-gnu',
  'darwin/x64': 'darwin-x64',
  'darwin/arm64': 'darwin-arm64',
  'win32/x64': 'win32-x64-msvc',
};
const target = targets[`${platform}/${arch}`];

if (!target || !supportedLibc) {
  throw new Error(`@anbani/native: unsupported platform ${platform}/${arch}/${libc}; see supported targets in README`);
}

const packageName = `@anbani/native-${target}`;
try {
  module.exports = require(packageName);
} catch (cause) {
  throw new Error(`@anbani/native: cannot load ${packageName}; reinstall with optional dependencies enabled`, { cause });
}
