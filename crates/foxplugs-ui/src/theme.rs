pub const FOXPLUGS_DARK_STYLESHEET: &str = r#"
* {
    background-color: #0d0c14;
    color: #d8d6e2;
    font-size: 13;
}

label {
    background-color: transparent;
    color: #d8d6e2;
}

label.title {
    font-size: 26;
    color: #b388ff;
    height: 48px;
    alignment: bottom-center;
}

label.subtitle {
    color: #6ea8fe;
    font-size: 13;
    height: 24px;
    alignment: center;
}

wave-preview {
    background-color: #12101c;
    border-color: #2a2638;
    border-width: 1px;
    corner-radius: 6px;
    child-space: 8px;
}

vstack.knob-grid {
    row-between: 14px;
    child-space: 16px;
    width: 1s;
}

hstack.knob-row {
    col-between: 18px;
    width: 1s;
    alignment: center;
}

vstack.knob-cell {
    width: 1s;
    row-between: 6px;
    alignment: top-center;
}

label.knob-name {
    color: #6ea8fe;
    font-size: 11;
    alignment: center;
}

label.knob-value {
    color: #d8d6e2;
    font-size: 12;
    alignment: center;
}

label.knob-value:hover {
    color: #b388ff;
}

textbox.knob-value-input {
    background-color: #1a1825;
    color: #d8d6e2;
    border-color: #b388ff;
    border-width: 1px;
    corner-radius: 3px;
    padding: 2px 4px;
    font-size: 12;
}

knob-dial {
    background-color: transparent;
}
"#;
