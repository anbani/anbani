export const parseCsv = (text) => {
  const lines = text.split(/\r?\n/).filter((line) => line.length > 0);
  const header = lines[0].split(",");
  return lines.slice(1).map((line) => {
    const cells = line.split(",");
    const row = {};
    header.forEach((key, i) => {
      row[key] = cells[i];
    });
    return row;
  });
};

export default { parseCsv };
