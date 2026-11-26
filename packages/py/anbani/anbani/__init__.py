from .core.converter import classify_text, convert, interpret

__all__ = ["convert", "interpret", "classify_text", "toolkit", "lorem", "morse"]

def __getattr__(name):
    if name in ("toolkit", "lorem", "morse"):
        import importlib

        module = importlib.import_module(f".{name}", __name__)
        globals()[name] = module
        return module
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
