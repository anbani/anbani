"""PDF / e-book text extraction.

PyMuPDF is imported lazily inside :func:`ebook2text` so that importing this
module (and the rest of ``anbani.nlp``) does not require the optional
``pymupdf`` dependency.
"""

_HYPHENS = (
    "-",
    "‐",
    "‑",
    "‒",
    "–",
    "—",
    "―",
    "−",
    "֊",
    "⁻",
    "₋",
    "⸺",
    "⸻",
    "﹘",
    "－",
    "﹣",
)

def _import_pymupdf():
    try:
        import pymupdf
        return pymupdf
    except ImportError:
        try:
            import fitz
            return fitz
        except ImportError as e:
            raise ImportError(
                "ebook2text requires the optional 'pymupdf' dependency. "
                "Install it with: pip install pymupdf"
            ) from e

def ebook2text(path):
    """Extract and lightly clean text from the PDF / e-book at ``path``.

    Returns the extracted text, or a ``"Error: ..."`` string if extraction
    fails. Raises :class:`ImportError` if PyMuPDF is not installed.
    """
    pymupdf = _import_pymupdf()

    try:
        doc = pymupdf.open(path)
        text = "".join(
            doc.load_page(p).get_text() for p in range(doc.page_count)
        )

        for hyphen in _HYPHENS:
            text = text.replace(hyphen + "\n", "")

        text = text.replace("\n", " ")
    except Exception as e:
        return "Error: " + str(e)

    return text
