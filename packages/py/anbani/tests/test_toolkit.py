from anbani import toolkit

def test_count_uppercases_and_filters():
    assert toolkit.count("აბ ბ") == {"Ა": 1, "Ბ": 2}

def test_frequency_denominator_includes_filtered_chars():
    freq = toolkit.frequency("აბ ბ")
    assert freq == {"Ა": 0.25, "Ბ": 0.5}

def test_friedman_zero_for_short_input():
    assert toolkit.friedman("") == 0
    assert toolkit.friedman("ა") == 0

def test_friedman_index_of_coincidence():
    assert abs(toolkit.friedman("ააბ") - (1 / 3)) < 1e-12

def test_custom_miss_filter():
    assert toolkit.count("abაბ", miss="[a-zA-Z]") == {"A": 1, "B": 1}
