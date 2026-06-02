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

vstack.foxshaper-root {
    row-between: 12px;
    child-space: 14px;
    width: 1s;
}

hstack.top-bar {
    width: 1s;
    height: 66px;
    child-left: 14px;
    child-right: 14px;
    col-between: 18px;
    alignment: center;
}

vstack.brand-block {
    width: 220px;
    row-between: 0px;
}

label.title {
    font-size: 26;
    color: #b388ff;
    height: 34px;
    alignment: bottom-left;
}

label.subtitle {
    color: #6ea8fe;
    font-size: 13;
    height: 22px;
    alignment: top-left;
}

label.top-hint {
    color: #6f6a82;
    font-size: 12;
    alignment: center-right;
    width: 1s;
}

wave-preview {
    background-color: #12101c;
    border-color: #2a2638;
    border-width: 1px;
    corner-radius: 8px;
    child-space: 8px;
}

hstack.panel-row {
    col-between: 12px;
    width: 1s;
}

vstack.control-panel {
    background-color: #12101c;
    border-color: #232034;
    border-width: 1px;
    corner-radius: 8px;
    child-space: 12px;
    row-between: 8px;
    width: 1s;
    height: 132px;
}

label.section-title {
    color: #b388ff;
    font-size: 13;
    height: 18px;
    alignment: center-left;
}

label.panel-help {
    color: #6f6a82;
    font-size: 10;
    height: 26px;
    alignment: top-left;
}

hstack.compact-knob-row {
    col-between: 10px;
    width: 1s;
    alignment: center;
}

hstack.mix-strip {
    background-color: #12101c;
    border-color: #232034;
    border-width: 1px;
    corner-radius: 8px;
    child-space: 12px;
    col-between: 16px;
    width: 1s;
    height: 104px;
    alignment: center;
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
    row-between: 5px;
    alignment: top-center;
}

label.knob-name {
    color: #6ea8fe;
    font-size: 10;
    alignment: center;
}

label.knob-value {
    color: #d8d6e2;
    font-size: 11;
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
