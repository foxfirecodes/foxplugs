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
    height: 56px;
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
    height: 30px;
    alignment: bottom-left;
}

label.subtitle {
    color: #6ea8fe;
    font-size: 13;
    height: 18px;
    alignment: top-left;
}

label.top-hint {
    color: #6f6a82;
    font-size: 12;
    alignment: right;
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
    height: 140px;
}

label.section-title {
    color: #b388ff;
    font-size: 13;
    height: 18px;
    alignment: left;
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

param-slider {
    background-color: transparent;
}

vstack.param-slider-shell {
    background-color: transparent;
    row-between: 4px;
}

hstack.param-slider-header {
    background-color: transparent;
    col-between: 8px;
    width: 1s;
}

label.param-slider-name {
    color: #6ea8fe;
    font-size: 11;
    width: 1s;
    alignment: left;
}

label.param-slider-value {
    color: #d8d6e2;
    font-size: 11;
    width: 70px;
    alignment: right;
}


param-stepper {
    background-color: transparent;
    width: 108px;
    height: 44px;
}

vstack.stepper-shell {
    background-color: transparent;
    row-between: 3px;
    width: 1s;
}

label.stepper-name {
    color: #6ea8fe;
    font-size: 10;
    height: 14px;
    alignment: left;
}

hstack.stepper-row {
    background-color: transparent;
    col-between: 3px;
    width: 1s;
    height: 24px;
}

label.stepper-button {
    background-color: #1a1825;
    border-color: #2a2638;
    border-width: 1px;
    corner-radius: 4px;
    color: #b388ff;
    width: 18px;
    height: 24px;
    alignment: center;
}

label.stepper-button:hover {
    border-color: #b388ff;
}

label.stepper-value {
    background-color: #15131f;
    border-color: #2a2638;
    border-width: 1px;
    corner-radius: 4px;
    color: #d8d6e2;
    font-size: 10;
    width: 1s;
    height: 24px;
    alignment: center;
}

param-toggle-group {
    background-color: transparent;
}

vstack.toggle-group-shell {
    background-color: transparent;
    row-between: 5px;
    width: 1s;
}

label.toggle-group-name {
    color: #6ea8fe;
    font-size: 11;
    height: 16px;
    alignment: left;
}

hstack.toggle-row {
    col-between: 4px;
    width: 1s;
    background-color: transparent;
}

label.toggle-option {
    background-color: #1a1825;
    border-color: #2a2638;
    border-width: 1px;
    corner-radius: 4px;
    color: #d8d6e2;
    font-size: 10;
    padding: 3px 6px;
    height: 24px;
    width: 1s;
    alignment: center;
}

label.toggle-option:hover {
    border-color: #b388ff;
    color: #b388ff;
}

hstack.mix-strip {
    background-color: #12101c;
    border-color: #232034;
    border-width: 1px;
    corner-radius: 8px;
    child-space: 12px;
    col-between: 16px;
    width: 1s;
    height: 88px;
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
