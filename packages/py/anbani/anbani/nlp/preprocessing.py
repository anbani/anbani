import re

def nested_tokenize_by_separators(text, first_separator='\n', second_separator=' '):
    text = text.split(first_separator)
    text = [
        [
            token
            for token in sentence.strip().split(second_separator)
            if token != ''
        ]
        for sentence in text
        if sentence != ''
    ]
    return text

def sentence_tokenize(text):
    text = re.sub(r"[^Ⴀ-ჿⴀ-ⴥᲐ-Ჿ\.\?\!]", ' ', text)
    text = re.sub(r"[\?\!]", '.', text)
    text = re.sub(r"\s+", ' ', text)

    text = text.lower()
    text = nested_tokenize_by_separators(text, '.')

    return text

def paragraph_tokenize(text):
    text = re.sub(r"[^Ⴀ-ჿⴀ-ⴥᲐ-Ჿ\n]", ' ', text)

    text = text.lower()
    text = nested_tokenize_by_separators(text, '\n')

    return text


def word_tokenize(text):
    text = re.sub(r"[^Ⴀ-ჿⴀ-ⴥᲐ-Ჿ]", ' ', text)
    text = re.sub(r"\s+", ' ', text)

    text = text.lower()
    text = [token for token in text.split(' ') if token != '']

    return text

def cleanup(text):
    text = re.sub(r"[^Ⴀ-ჿⴀ-ⴥᲐ-Ჿ\.\?\!\,]", ' ', text)
    text = re.sub(r"\s+", ' ', text)

    text = text.lower()
    return text
