import contractionsCsv from "../../data/georgian_contractions.csv";

export const readContractionsCsv = () => contractionsCsv;

export const readAmbigramCsv = () => {
  throw new Error(
    "Balanced/accurate georgianisation needs the ~400KB ambigram table, which " +
      "isn't bundled for the browser. Use georgianise(text, 'fast'), or run in " +
      "Node (import from 'anbani')."
  );
};

export default { readContractionsCsv, readAmbigramCsv };
