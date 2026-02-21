use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::event::Event;
use crate::reader::LogReader;

#[derive(Deserialize)]
pub struct EventQueryParams {
    filter: Option<String>,
    #[serde(rename = "type")]
    event_type: Option<String>,
}

pub async fn index() -> HttpResponse {
    let html = r##"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <title>Black Box</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="description" content="This server remembers what just happened.">
    <meta property="og:description" content="This server remembers what just happened.">
    <meta name="theme-color" content="#ffffff">
    <link rel="icon" type="image/svg+xml"
      href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'%3E%3Crect x='10' y='10' width='80' height='80' fill='black'/%3E%3C/svg%3E">
    <style>
        /*! tailwindcss v4.1.18 | MIT License | https://tailwindcss.com */
@layer properties{@supports (((-webkit-hyphens:none)) and (not (margin-trim:inline))) or ((-moz-orient:inline) and (not (color:rgb(from red r g b)))){*,:before,:after,::backdrop{--tw-border-style:solid;--tw-font-weight:initial;--tw-shadow:0 0 #0000;--tw-shadow-color:initial;--tw-shadow-alpha:100%;--tw-inset-shadow:0 0 #0000;--tw-inset-shadow-color:initial;--tw-inset-shadow-alpha:100%;--tw-ring-color:initial;--tw-ring-shadow:0 0 #0000;--tw-inset-ring-color:initial;--tw-inset-ring-shadow:0 0 #0000;--tw-ring-inset:initial;--tw-ring-offset-width:0px;--tw-ring-offset-color:#fff;--tw-ring-offset-shadow:0 0 #0000;--tw-blur:initial;--tw-brightness:initial;--tw-contrast:initial;--tw-grayscale:initial;--tw-hue-rotate:initial;--tw-invert:initial;--tw-opacity:initial;--tw-saturate:initial;--tw-sepia:initial;--tw-drop-shadow:initial;--tw-drop-shadow-color:initial;--tw-drop-shadow-alpha:100%;--tw-drop-shadow-size:initial;--tw-backdrop-blur:initial;--tw-backdrop-brightness:initial;--tw-backdrop-contrast:initial;--tw-backdrop-grayscale:initial;--tw-backdrop-hue-rotate:initial;--tw-backdrop-invert:initial;--tw-backdrop-opacity:initial;--tw-backdrop-saturate:initial;--tw-backdrop-sepia:initial;--tw-duration:initial}}}@layer theme{:root,:host{--font-sans:ui-sans-serif,system-ui,sans-serif,"Apple Color Emoji","Segoe UI Emoji","Segoe UI Symbol","Noto Color Emoji";--font-mono:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,"Liberation Mono","Courier New",monospace;--color-red-500:oklch(63.7% .237 25.331);--color-red-600:oklch(57.7% .245 27.325);--color-yellow-500:oklch(79.5% .184 86.047);--color-yellow-600:oklch(68.1% .162 75.834);--color-green-500:oklch(72.3% .219 149.579);--color-green-600:oklch(62.7% .194 149.214);--color-blue-600:oklch(54.6% .245 262.881);--color-gray-50:oklch(98.5% .002 247.839);--color-gray-200:oklch(92.8% .006 264.531);--color-gray-300:oklch(87.2% .01 258.338);--color-gray-400:oklch(70.7% .022 261.325);--color-gray-500:oklch(55.1% .027 264.364);--color-gray-600:oklch(44.6% .03 256.802);--color-gray-700:oklch(37.3% .034 259.733);--color-gray-800:oklch(27.8% .033 256.848);--color-gray-900:oklch(21% .034 264.665);--color-white:#fff;--spacing:.25rem;--text-xs:.75rem;--text-xs--line-height:calc(1/.75);--font-weight-normal:400;--font-weight-medium:500;--font-weight-semibold:600;--default-transition-duration:.15s;--default-transition-timing-function:cubic-bezier(.4,0,.2,1);--default-font-family:var(--font-sans);--default-mono-font-family:var(--font-mono)}}@layer base{*,:after,:before,::backdrop{box-sizing:border-box;border:0 solid;margin:0;padding:0}::file-selector-button{box-sizing:border-box;border:0 solid;margin:0;padding:0}html,:host{-webkit-text-size-adjust:100%;tab-size:4;line-height:1.5;font-family:var(--default-font-family,ui-sans-serif,system-ui,sans-serif,"Apple Color Emoji","Segoe UI Emoji","Segoe UI Symbol","Noto Color Emoji");font-feature-settings:var(--default-font-feature-settings,normal);font-variation-settings:var(--default-font-variation-settings,normal);-webkit-tap-highlight-color:transparent}hr{height:0;color:inherit;border-top-width:1px}abbr:where([title]){-webkit-text-decoration:underline dotted;text-decoration:underline dotted}h1,h2,h3,h4,h5,h6{font-size:inherit;font-weight:inherit}a{color:inherit;-webkit-text-decoration:inherit;-webkit-text-decoration:inherit;-webkit-text-decoration:inherit;text-decoration:inherit}b,strong{font-weight:bolder}code,kbd,samp,pre{font-family:var(--default-mono-font-family,ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,"Liberation Mono","Courier New",monospace);font-feature-settings:var(--default-mono-font-feature-settings,normal);font-variation-settings:var(--default-mono-font-variation-settings,normal);font-size:1em}small{font-size:80%}sub,sup{vertical-align:baseline;font-size:75%;line-height:0;position:relative}sub{bottom:-.25em}sup{top:-.5em}table{text-indent:0;border-color:inherit;border-collapse:collapse}:-moz-focusring{outline:auto}progress{vertical-align:baseline}summary{display:list-item}ol,ul,menu{list-style:none}img,svg,video,canvas,audio,iframe,embed,object{vertical-align:middle;display:block}img,video{max-width:100%;height:auto}button,input,select,optgroup,textarea{font:inherit;font-feature-settings:inherit;font-variation-settings:inherit;letter-spacing:inherit;color:inherit;opacity:1;background-color:#0000;border-radius:0}::file-selector-button{font:inherit;font-feature-settings:inherit;font-variation-settings:inherit;letter-spacing:inherit;color:inherit;opacity:1;background-color:#0000;border-radius:0}:where(select:is([multiple],[size])) optgroup{font-weight:bolder}:where(select:is([multiple],[size])) optgroup option{padding-inline-start:20px}::file-selector-button{margin-inline-end:4px}::placeholder{opacity:1}@supports (not ((-webkit-appearance:-apple-pay-button))) or (contain-intrinsic-size:1px){::placeholder{color:currentColor}@supports (color:color-mix(in lab, red, red)){::placeholder{color:color-mix(in oklab,currentcolor 50%,transparent)}}}textarea{resize:vertical}::-webkit-search-decoration{-webkit-appearance:none}::-webkit-date-and-time-value{min-height:1lh;text-align:inherit}::-webkit-datetime-edit{display:inline-flex}::-webkit-datetime-edit-fields-wrapper{padding:0}::-webkit-datetime-edit{padding-block:0}::-webkit-datetime-edit-year-field{padding-block:0}::-webkit-datetime-edit-month-field{padding-block:0}::-webkit-datetime-edit-day-field{padding-block:0}::-webkit-datetime-edit-hour-field{padding-block:0}::-webkit-datetime-edit-minute-field{padding-block:0}::-webkit-datetime-edit-second-field{padding-block:0}::-webkit-datetime-edit-millisecond-field{padding-block:0}::-webkit-datetime-edit-meridiem-field{padding-block:0}::-webkit-calendar-picker-indicator{line-height:1}:-moz-ui-invalid{box-shadow:none}button,input:where([type=button],[type=reset],[type=submit]){appearance:button}::file-selector-button{appearance:button}::-webkit-inner-spin-button{height:auto}::-webkit-outer-spin-button{height:auto}[hidden]:where(:not([hidden=until-found])){display:none!important}}@layer components;@layer utilities{.absolute{position:absolute}.fixed{position:fixed}.relative{position:relative}.static{position:static}.inset-0{inset:calc(var(--spacing)*0)}.top-0{top:calc(var(--spacing)*0)}.left-0{left:calc(var(--spacing)*0)}.z-10{z-index:10}.container{width:100%}@media (min-width:40rem){.container{max-width:40rem}}@media (min-width:48rem){.container{max-width:48rem}}@media (min-width:64rem){.container{max-width:64rem}}@media (min-width:80rem){.container{max-width:80rem}}@media (min-width:96rem){.container{max-width:96rem}}.mx-auto{margin-inline:auto}.mt-1{margin-top:calc(var(--spacing)*1)}.mr-1{margin-right:calc(var(--spacing)*1)}.ml-1{margin-left:calc(var(--spacing)*1)}.ml-2{margin-left:calc(var(--spacing)*2)}.ml-auto{margin-left:auto}.block{display:block}.contents{display:contents}.flex{display:flex}.grid{display:grid}.hidden{display:none}.inline{display:inline}.inline-block{display:inline-block}.table{display:table}.size-4{width:calc(var(--spacing)*4);height:calc(var(--spacing)*4)}.h-3{height:calc(var(--spacing)*3)}.h-4{height:calc(var(--spacing)*4)}.h-12{height:calc(var(--spacing)*12)}.h-full{height:100%}.max-h-96{max-height:calc(var(--spacing)*96)}.min-h-screen{min-height:100vh}.w-10{width:calc(var(--spacing)*10)}.w-16{width:calc(var(--spacing)*16)}.w-32{width:calc(var(--spacing)*32)}.w-full{width:100%}.flex-1{flex:1}.grow{flex-grow:1}.cursor-pointer{cursor:pointer}.resize{resize:both}.grid-cols-2{grid-template-columns:repeat(2,minmax(0,1fr))}.flex-col{flex-direction:column}.items-center{align-items:center}.items-end{align-items:flex-end}.justify-between{justify-content:space-between}.justify-center{justify-content:center}.gap-1{gap:calc(var(--spacing)*1)}.gap-3{gap:calc(var(--spacing)*3)}.gap-4{gap:calc(var(--spacing)*4)}.gap-x-4{column-gap:calc(var(--spacing)*4)}.overflow-hidden{overflow:hidden}.overflow-visible{overflow:visible}.overflow-y-auto{overflow-y:auto}.rounded{border-radius:.25rem}.border{border-style:var(--tw-border-style);border-width:1px}.border-b{border-bottom-style:var(--tw-border-style);border-bottom-width:1px}.border-l{border-left-style:var(--tw-border-style);border-left-width:1px}.border-gray-200{border-color:var(--color-gray-200)}.border-gray-300{border-color:var(--color-gray-300)}.bg-gray-50{background-color:var(--color-gray-50)}.bg-gray-200{background-color:var(--color-gray-200)}.bg-green-500{background-color:var(--color-green-500)}.bg-red-500{background-color:var(--color-red-500)}.bg-white{background-color:var(--color-white)}.bg-yellow-500{background-color:var(--color-yellow-500)}.p-2{padding:calc(var(--spacing)*2)}.px-1{padding-inline:calc(var(--spacing)*1)}.px-2{padding-inline:calc(var(--spacing)*2)}.px-4{padding-inline:calc(var(--spacing)*4)}.px-5{padding-inline:calc(var(--spacing)*5)}.py-0{padding-block:calc(var(--spacing)*0)}.py-0\.5{padding-block:calc(var(--spacing)*.5)}.py-2{padding-block:calc(var(--spacing)*2)}.py-\[80px\]{padding-block:80px}.pr-2{padding-right:calc(var(--spacing)*2)}.text-left{text-align:left}.text-right{text-align:right}.align-middle{vertical-align:middle}.font-mono{font-family:var(--font-mono)}.text-xs{font-size:var(--text-xs);line-height:var(--tw-leading,var(--text-xs--line-height))}.font-medium{--tw-font-weight:var(--font-weight-medium);font-weight:var(--font-weight-medium)}.font-normal{--tw-font-weight:var(--font-weight-normal);font-weight:var(--font-weight-normal)}.font-semibold{--tw-font-weight:var(--font-weight-semibold);font-weight:var(--font-weight-semibold)}.break-all{word-break:break-all}.whitespace-nowrap{white-space:nowrap}.text-blue-600{color:var(--color-blue-600)}.text-gray-400{color:var(--color-gray-400)}.text-gray-500{color:var(--color-gray-500)}.text-gray-500\/60{color:#6a728299}@supports (color:color-mix(in lab, red, red)){.text-gray-500\/60{color:color-mix(in oklab,var(--color-gray-500)60%,transparent)}}.text-gray-600{color:var(--color-gray-600)}.text-gray-700{color:var(--color-gray-700)}.text-gray-800{color:var(--color-gray-800)}.text-gray-900{color:var(--color-gray-900)}.text-green-600{color:var(--color-green-600)}.text-red-600{color:var(--color-red-600)}.text-yellow-600{color:var(--color-yellow-600)}.ring{--tw-ring-shadow:var(--tw-ring-inset,)0 0 0 calc(1px + var(--tw-ring-offset-width))var(--tw-ring-color,currentcolor);box-shadow:var(--tw-inset-shadow),var(--tw-inset-ring-shadow),var(--tw-ring-offset-shadow),var(--tw-ring-shadow),var(--tw-shadow)}.blur{--tw-blur:blur(8px);filter:var(--tw-blur,)var(--tw-brightness,)var(--tw-contrast,)var(--tw-grayscale,)var(--tw-hue-rotate,)var(--tw-invert,)var(--tw-saturate,)var(--tw-sepia,)var(--tw-drop-shadow,)}.\!filter{filter:var(--tw-blur,)var(--tw-brightness,)var(--tw-contrast,)var(--tw-grayscale,)var(--tw-hue-rotate,)var(--tw-invert,)var(--tw-saturate,)var(--tw-sepia,)var(--tw-drop-shadow,)!important}.filter{filter:var(--tw-blur,)var(--tw-brightness,)var(--tw-contrast,)var(--tw-grayscale,)var(--tw-hue-rotate,)var(--tw-invert,)var(--tw-saturate,)var(--tw-sepia,)var(--tw-drop-shadow,)}.backdrop-filter{-webkit-backdrop-filter:var(--tw-backdrop-blur,)var(--tw-backdrop-brightness,)var(--tw-backdrop-contrast,)var(--tw-backdrop-grayscale,)var(--tw-backdrop-hue-rotate,)var(--tw-backdrop-invert,)var(--tw-backdrop-opacity,)var(--tw-backdrop-saturate,)var(--tw-backdrop-sepia,);backdrop-filter:var(--tw-backdrop-blur,)var(--tw-backdrop-brightness,)var(--tw-backdrop-contrast,)var(--tw-backdrop-grayscale,)var(--tw-backdrop-hue-rotate,)var(--tw-backdrop-invert,)var(--tw-backdrop-opacity,)var(--tw-backdrop-saturate,)var(--tw-backdrop-sepia,)}.transition{transition-property:color,background-color,border-color,outline-color,text-decoration-color,fill,stroke,--tw-gradient-from,--tw-gradient-via,--tw-gradient-to,opacity,box-shadow,transform,translate,scale,rotate,filter,-webkit-backdrop-filter,backdrop-filter,display,content-visibility,overlay,pointer-events;transition-timing-function:var(--tw-ease,var(--default-transition-timing-function));transition-duration:var(--tw-duration,var(--default-transition-duration))}.transition-all{transition-property:all;transition-timing-function:var(--tw-ease,var(--default-transition-timing-function));transition-duration:var(--tw-duration,var(--default-transition-duration))}.duration-100{--tw-duration:.1s;transition-duration:.1s}.duration-300{--tw-duration:.3s;transition-duration:.3s}@media (hover:hover){.hover\:text-gray-600:hover{color:var(--color-gray-600)}.hover\:text-gray-700:hover{color:var(--color-gray-700)}}.focus\:ring-1:focus{--tw-ring-shadow:var(--tw-ring-inset,)0 0 0 calc(1px + var(--tw-ring-offset-width))var(--tw-ring-color,currentcolor);box-shadow:var(--tw-inset-shadow),var(--tw-inset-ring-shadow),var(--tw-ring-offset-shadow),var(--tw-ring-shadow),var(--tw-shadow)}.focus\:ring-gray-400:focus{--tw-ring-color:var(--color-gray-400)}.focus\:outline-none:focus{--tw-outline-style:none;outline-style:none}}@property --tw-border-style{syntax:"*";inherits:false;initial-value:solid}@property --tw-font-weight{syntax:"*";inherits:false}@property --tw-shadow{syntax:"*";inherits:false;initial-value:0 0 #0000}@property --tw-shadow-color{syntax:"*";inherits:false}@property --tw-shadow-alpha{syntax:"<percentage>";inherits:false;initial-value:100%}@property --tw-inset-shadow{syntax:"*";inherits:false;initial-value:0 0 #0000}@property --tw-inset-shadow-color{syntax:"*";inherits:false}@property --tw-inset-shadow-alpha{syntax:"<percentage>";inherits:false;initial-value:100%}@property --tw-ring-color{syntax:"*";inherits:false}@property --tw-ring-shadow{syntax:"*";inherits:false;initial-value:0 0 #0000}@property --tw-inset-ring-color{syntax:"*";inherits:false}@property --tw-inset-ring-shadow{syntax:"*";inherits:false;initial-value:0 0 #0000}@property --tw-ring-inset{syntax:"*";inherits:false}@property --tw-ring-offset-width{syntax:"<length>";inherits:false;initial-value:0}@property --tw-ring-offset-color{syntax:"*";inherits:false;initial-value:#fff}@property --tw-ring-offset-shadow{syntax:"*";inherits:false;initial-value:0 0 #0000}@property --tw-blur{syntax:"*";inherits:false}@property --tw-brightness{syntax:"*";inherits:false}@property --tw-contrast{syntax:"*";inherits:false}@property --tw-grayscale{syntax:"*";inherits:false}@property --tw-hue-rotate{syntax:"*";inherits:false}@property --tw-invert{syntax:"*";inherits:false}@property --tw-opacity{syntax:"*";inherits:false}@property --tw-saturate{syntax:"*";inherits:false}@property --tw-sepia{syntax:"*";inherits:false}@property --tw-drop-shadow{syntax:"*";inherits:false}@property --tw-drop-shadow-color{syntax:"*";inherits:false}@property --tw-drop-shadow-alpha{syntax:"<percentage>";inherits:false;initial-value:100%}@property --tw-drop-shadow-size{syntax:"*";inherits:false}@property --tw-backdrop-blur{syntax:"*";inherits:false}@property --tw-backdrop-brightness{syntax:"*";inherits:false}@property --tw-backdrop-contrast{syntax:"*";inherits:false}@property --tw-backdrop-grayscale{syntax:"*";inherits:false}@property --tw-backdrop-hue-rotate{syntax:"*";inherits:false}@property --tw-backdrop-invert{syntax:"*";inherits:false}@property --tw-backdrop-opacity{syntax:"*";inherits:false}@property --tw-backdrop-saturate{syntax:"*";inherits:false}@property --tw-backdrop-sepia{syntax:"*";inherits:false}@property --tw-duration{syntax:"*";inherits:false}
        /* Custom overrides */
        * { line-height: 1.5; }
        body { font-size: 13px; }
        .max-w { max-width: 32rem; }
        th, td { padding: 0; }
        .backdrop-blur-10xl { -webkit-backdrop-filter: blur(1000px); backdrop-filter: blur(1000px); }
        /* Loading spinner for timeline data fetch */
        @keyframes spin {
            from { transform: rotate(0deg); }
            to { transform: rotate(360deg); }
        }
        .loading-spinner {
            width: 12px;
            height: 12px;
            border: 2px solid rgba(156, 163, 175, 0.3);
            border-top-color: #9ca3af;
            border-radius: 50%;
            animation: spin 0.8s linear infinite;
        }
    </style>
</head>
<body class="bg-gray-50 min-h-screen">
<div class="max-w mx-auto px-4 py-[80px]">
    <div class="fixed w-full z-10 left-0 top-0 flex backdrop-blur-10xl">
        <div class="grow">
            <canvas id="timelineChart" class="w-full h-12 cursor-pointer rounded" style="opacity:0;background:transparent;transition:opacity 0.3s ease-in;" title="Click to jump to a point in time"></canvas>
        </div>
        <div class="flex gap-3 px-5 py-2 text-gray-400 items-center">
            <svg id="rewindBtn" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="size-4 hover:text-gray-600 transition duration-100 cursor-pointer" title="Rewind 1 minute">
                <path d="M7.712 4.818A1.5 1.5 0 0 1 10 6.095v2.972c.104-.13.234-.248.389-.343l6.323-3.906A1.5 1.5 0 0 1 19 6.095v7.81a1.5 1.5 0 0 1-2.288 1.276l-6.323-3.905a1.505 1.505 0 0 1-.389-.344v2.973a1.5 1.5 0 0 1-2.288 1.276l-6.323-3.905a1.5 1.5 0 0 1 0-2.552l6.323-3.906Z" />
            </svg>
            <svg id="pauseBtn" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="size-4 hover:text-gray-600 transition duration-100 cursor-pointer" title="Pause (enable time-travel)">
                <path d="M5.75 3a.75.75 0 0 0-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 0 0 .75-.75V3.75A.75.75 0 0 0 7.25 3h-1.5ZM12.75 3a.75.75 0 0 0-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 0 0 .75-.75V3.75a.75.75 0 0 0-.75-.75h-1.5Z" />
            </svg>
            <svg id="playBtn" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="size-4 text-gray-800 hover:text-gray-600 transition duration-100 cursor-pointer" style="display:none" title="Resume live view">
                <path d="M6.3 2.84A1.5 1.5 0 0 0 4 4.11v11.78a1.5 1.5 0 0 0 2.3 1.27l9.344-5.891a1.5 1.5 0 0 0 0-2.538L6.3 2.841Z" />
            </svg>
            <svg id="fastForwardBtn" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="size-4 hover:text-gray-600 transition duration-100 cursor-pointer" title="Fast forward 1 minute">
                <path d="M3.288 4.818A1.5 1.5 0 0 0 1 6.095v7.81a1.5 1.5 0 0 0 2.288 1.276l6.323-3.905c.155-.096.285-.213.389-.344v2.973a1.5 1.5 0 0 0 2.288 1.276l6.323-3.905a1.5 1.5 0 0 0 0-2.552l-6.323-3.906A1.5 1.5 0 0 0 10 6.095v2.972a1.506 1.506 0 0 0-.389-.343L3.288 4.818Z" />
            </svg>
            <div class="border-l border-gray-300 h-4"></div>
            <div class="flex flex-col text-xs items-end relative">
                <input type="datetime-local" id="timePicker" class="absolute top-0 right-0 px-1 py-0.5 border border-gray-300 rounded text-gray-700 text-xs bg-white" style="display:none;z-index:20;" title="Select a specific date and time to view" />
                <span id="timeDisplay" class="cursor-pointer hover:text-gray-700 whitespace-nowrap" style="color:#ef4444;" title="Click to select time, Shift+Click to go Live">Disconnected</span>
                <span id="timeRange" class="text-gray-400 text-xs whitespace-nowrap" title="Total duration of recorded history"></span>
            </div>
        </div>
    </div>
    <div id="mainContent" style="display:none;">
    <div class="flex justify-between items-center">
        <div class="text-gray-900 font-semibold" title="Black Box">Black Box</div>
        <div id="headerControlsWrapper">
            <div id="headerControls" class="flex items-center gap-1 text-gray-400">
                <div id="playbackTimeDisplay" class="flex items-center gap-1 text-xs mr-1" style="display:none;color:#f59e0b;" title="Viewing historical data at this time">
                    <span id="playbackTime" style="display: none;"></span>
                </div>
                <div id="timelineLoadingSpinner" class="loading-spinner" style="display:none;"></div>
                <svg id="headerRewindBtn" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="hover:text-gray-600 transition duration-100 cursor-pointer" style="width:14px;height:14px" title="Rewind 1 minute">
                    <path d="M7.712 4.818A1.5 1.5 0 0 1 10 6.095v2.972c.104-.13.234-.248.389-.343l6.323-3.906A1.5 1.5 0 0 1 19 6.095v7.81a1.5 1.5 0 0 1-2.288 1.276l-6.323-3.905a1.505 1.505 0 0 1-.389-.344v2.973a1.5 1.5 0 0 1-2.288 1.276l-6.323-3.905a1.5 1.5 0 0 1 0-2.552l6.323-3.906Z" />
                </svg>
                <svg id="headerPauseBtn" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="hover:text-gray-600 transition duration-100 cursor-pointer" style="width:14px;height:14px" title="Pause (enable time-travel)">
                    <path d="M5.75 3a.75.75 0 0 0-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 0 0 .75-.75V3.75A.75.75 0 0 0 7.25 3h-1.5ZM12.75 3a.75.75 0 0 0-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 0 0 .75-.75V3.75a.75.75 0 0 0-.75-.75h-1.5Z" />
                </svg>
                <svg id="headerPlayBtn" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="text-gray-800 hover:text-gray-600 transition duration-100 cursor-pointer" style="width:14px;height:14px;display:none" title="Resume live view">
                    <path d="M6.3 2.84A1.5 1.5 0 0 0 4 4.11v11.78a1.5 1.5 0 0 0 2.3 1.27l9.344-5.891a1.5 1.5 0 0 0 0-2.538L6.3 2.841Z" />
                </svg>
                <svg id="headerFastForwardBtn" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="hover:text-gray-600 transition duration-100 cursor-pointer" style="width:14px;height:14px" title="Fast forward 1 minute">
                    <path d="M3.288 4.818A1.5 1.5 0 0 0 1 6.095v7.81a1.5 1.5 0 0 0 2.288 1.276l6.323-3.905c.155-.096.285-.213.389-.344v2.973a1.5 1.5 0 0 0 2.288 1.276l6.323-3.905a1.5 1.5 0 0 0 0-2.552l-6.323-3.906A1.5 1.5 0 0 0 10 6.095v2.972a1.506 1.506 0 0 0-.389-.343L3.288 4.818Z" />
                </svg>
            </div>
            <span id="headerDisconnected" class="text-xs" style="display:none;color:#ef4444;">Disconnected</span>
        </div>
    </div>
    <div class="flex justify-between text-gray-500">
        <span id="datetime" title="System date and time"></span>
        <span id="uptime" title="System uptime"></span>
    </div>
    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">System</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div id="kernelRow" class="text-gray-500" title="Kernel version"></div>
    <div id="cpuDetailsRow" class="text-gray-500" title="CPU model and clock speed"></div>
    <div class="text-gray-500 flex items-center gap-4">
        <div class="flex-1 flex items-center gap-4">
            <span class="w-10" title="Total CPU usage across all cores">CPU</span>
            <span class="relative flex-1 bg-gray-200" style="height:10px;border-radius:1px">
                <span id="cpuBar" class="block h-full transition-all duration-300" style="width:0%;border-radius:1px"></span>
                <span id="cpuPct" class="absolute inset-0 flex items-center justify-center text-gray-500/60 overflow-visible"></span>
            </span>
        </div>
        <span id="loadVal" class="flex-1 text-right text-gray-500" title="1, 5, and 15 minute load averages">Load average: --% --% --%</span>
    </div>
    <div id="cpuCoresContainer" class="grid grid-cols-2 gap-x-4" title="Usage breakdown by CPU core"></div>
    <div class="flex items-center" style="height:19.5px;width:100%;">
        <canvas id="cpuChart" style="height:10px;width:100%;" title="CPU usage history (60s)"></canvas>
    </div>
    <div class="flex justify-between gap-4">
        <div class="text-gray-500 flex-1" id="ramUsed" title="RAM in use"></div>
        <div class="text-gray-500 flex-1 text-right" id="cpuTemp" title="CPU package temperature"></div>
    </div>
    <div class="flex justify-between gap-4">
        <div class="text-gray-500 flex-1" id="ramAvail" title="RAM available"></div>
        <div class="text-gray-500 flex-1 text-right" id="moboTemp" title="Motherboard temperature"></div>
    </div>
    <div class="flex items-center" style="height:19.5px;width:100%;">
        <canvas id="memoryChart" style="height:10px;width:100%;" title="Memory usage history (60s)"></canvas>
    </div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold" id="graphicsSection" style="display:none" title="GPU metrics">
        <span class="pr-2">Graphics</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div class="flex justify-between gap-4" id="graphicsRow1" style="display:none">
        <div class="text-gray-500" id="gpuFreq" title="GPU clock speed"></div>
        <div class="text-gray-500 text-right" id="gpuTemp" title="GPU temperature"></div>
    </div>
    <div class="flex justify-between gap-4" id="graphicsRow2" style="display:none">
        <div class="text-gray-500" id="memFreq" title="VRAM clock speed"></div>
        <div class="text-gray-500 text-right" id="imgQuality" title="GPU power draw"></div>
    </div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">Network</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div class="text-gray-500 flex gap-4">
        <div class="flex-1">
            <div>
                <span id="netName" title="Network interface name"></span>
                <span id="netSpeedDown" title="Current download rate"></span>
            </div>
            <div class="flex items-center" style="height:19.5px;width:100%;">
                <canvas id="netDownChart" style="height:10px;width:100%;" title="Download rate history (60s)"></canvas>
            </div>
        </div>
        <div class="flex-1">
            <div id="netSpeedUp" title="Current upload rate"></div>
            <div class="flex items-center" style="height:19.5px;width:100%;">
                <canvas id="netUpChart" style="height:10px;width:100%;" title="Upload rate history (60s)"></canvas>
            </div>
        </div>
    </div>
    <div class="text-gray-500 flex gap-4">
        <span class="flex-1" id="netRxStats" title="RX errors and drops per second"></span>
        <span class="flex-1" id="netTxStats" title="TX errors and drops per second"></span>
    </div>
    <div class="grid grid-cols-2 gap-x-4 text-gray-500">
        <div id="netAddress" title="Interface IP address"></div>
        <div id="netTcp" title="Active TCP connections"></div>
        <div id="netGateway" title="Default gateway"></div>
        <div id="netDns" title="DNS server"></div>
    </div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2" title="Mounted filesystem usage">Storage</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div id="diskContainer" title="Disk space used per mount point"></div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold" id="diskIoSection" style="display:none" title="Read/write throughput per block device">
        <span class="pr-2">Disk IO</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <table class="w-full text-gray-500" id="diskIoTable" style="display:none">
        <thead><tr class="text-left text-gray-400">
            <th class="font-normal" style="width:60px" title="Device">Device</th>
            <th class="font-normal text-right" style="width:80px" title="Read throughput">Read</th>
            <th class="font-normal text-right" style="width:80px" title="Write throughput">Write</th>
            <th class="font-normal text-right" style="width:50px" title="Drive temperature">Temp</th>
            <th style="width:128px" title="I/O activity (60s)"></th>
        </tr></thead>
        <tbody id="diskIoTableBody"></tbody>
    </table>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">Processes</span>
        <span id="procCount" class="text-gray-500 font-normal pr-2" title="Total and running process count"></span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <table class="w-full text-gray-500" title="Processes sorted by CPU usage">
        <thead><tr class="text-left text-gray-400">
            <th class="font-medium text-gray-700">Top CPU</th>
            <th class="font-normal w-16" title="Owner">User</th>
            <th class="font-normal w-16" title="Process ID (PID)">PID</th>
            <th class="font-normal w-16 text-right" title="CPU usage">CPU%</th>
            <th class="font-normal w-16 text-right" title="Memory usage">MEM%</th>
        </tr></thead>
        <tbody id="topCpuTable"></tbody>
    </table>
    <table class="w-full text-gray-500" title="Processes sorted by memory usage">
        <thead><tr class="text-left text-gray-400">
            <th class="font-medium text-gray-700">Top Memory</th>
            <th class="font-normal w-16" title="Owner">User</th>
            <th class="font-normal w-16" title="Process ID (PID)">PID</th>
            <th class="font-normal w-16 text-right" title="CPU usage">CPU%</th>
            <th class="font-normal w-16 text-right" title="Memory usage">MEM%</th>
        </tr></thead>
        <tbody id="topMemTable"></tbody>
    </table>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold" id="usersSection" style="display:none" title="Logged in users">
        <span class="pr-2">Users</span>
        <span id="userCount" class="text-gray-500 font-normal pr-2" title="User count"></span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div id="usersContainer"></div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2" title="Process, security, and system events">Events</span>
        <div class="flex-1 flex items-center">
            <div class="flex-1 border-b border-gray-200"></div>
            <div class="flex gap-1 items-center font-normal ml-2">
                <input type="text" id="filterInput" placeholder="Search..." title="Search events"
                    class="px-2 py-0 border border-gray-300 rounded text-gray-700 focus:outline-none focus:ring-1 focus:ring-gray-400" />
                <select id="eventType" class="px-2 py-0 border border-gray-300 rounded text-gray-700 focus:outline-none" title="Show only this event type">
                    <option value="">All</option>
                    <option value="process">Process</option>
                    <option value="security">Security</option>
                    <option value="anomaly">Anomaly</option>
                    <option value="filesystem">File System</option>
                </select>
            </div>
        </div>
    </div>
    <div id="eventsContainer" class="font-mono max-h-96 p-2 overflow-y-auto bg-white border border-gray-200 rounded mt-1" style="font-size:12px; min-height: 384px;" title="Last 1000 events"></div>
    </div>
</div>

<script>
let ws=null, eventBuffer=[], lastStats=null, isPaused=false;
const MAX_BUFFER=1000;
const eventKeys = new Set(); // Track unique event keys for deduplication (O(1) lookup)
const memoryHistory = []; // Track last 60 seconds of memory usage
const cpuHistory = []; // Track last 60 seconds of CPU usage
const netDownHistory = []; // Track last 60 seconds of download speed
const netUpHistory = []; // Track last 60 seconds of upload speed
const diskIoHistoryMap = {}; // Track last 60 seconds per disk
const MAX_HISTORY = 60;

// Cache for static/semi-static fields (these may not be in every event)
let cachedMemTotal = null;
let cachedSwapTotal = null;
let cachedDiskTotal = null;
let cachedFilesystems = [];
let cachedNetIp = null;
let cachedNetGateway = null;
let cachedNetDns = null;
let cachedKernel = null;
let cachedCpuModel = null;
let cachedCpuMhz = null;
let cachedProcesses = [];
let cachedTotalProcesses = null;
let cachedRunningProcesses = null;

// Previous values cache for change detection (optimization to avoid unnecessary DOM updates)
const prevValues = {};
let prevValueCleanupCounter = 0;

// Periodically clean up prevValues to prevent memory leak
function cleanupPrevValues() {
    const keys = Object.keys(prevValues);
    // Only keep entries for elements that still exist in the DOM
    keys.forEach(key => {
        // Extract the actual element ID (remove suffixes like _text, _html, _style_, _class)
        const baseId = key.split('_')[0];
        if (!document.getElementById(baseId) && !document.getElementById(key)) {
            delete prevValues[key];
        }
    });
}

// ===== Performance Optimizations for WebSocket Updates (1Hz) =====
// These optimizations ensure smooth 1-second updates without stressing system resources:
// 1. requestAnimationFrame batching - all canvas redraws happen in single animation frame
// 2. Canvas context caching - avoid repeated getContext() calls
// 3. Batch fillRect by color - reduce canvas state changes (huge performance gain)
// 4. DOM element caching - reuse table rows instead of recreating
// 5. Change detection - only update DOM when values actually change
// 6. Document fragments - batch DOM insertions to avoid reflows
// 7. Alpha channel disabled - faster canvas rendering
// 8. Switch statements - faster than if-else chains

// Performance optimization: batch chart updates with requestAnimationFrame
let chartUpdateQueued = false;
let chartsNeedingUpdate = new Set();

function queueChartUpdate(chartId) {
    chartsNeedingUpdate.add(chartId);
    if (!chartUpdateQueued) {
        chartUpdateQueued = true;
        requestAnimationFrame(() => {
            chartsNeedingUpdate.forEach(id => {
                switch(id) {
                    case 'cpu': drawChart('cpuChart', cpuHistory); break;
                    case 'memory': drawChart('memoryChart', memoryHistory); break;
                    case 'netDown': drawNetworkChart('netDownChart', netDownHistory); break;
                    case 'netUp': drawNetworkChart('netUpChart', netUpHistory); break;
                }
            });
            chartsNeedingUpdate.clear();
            chartUpdateQueued = false;
        });
    }
}

// Cache canvas contexts to avoid repeated getContext calls
const canvasContextCache = {};

// Helper function to update DOM element only if value changed
function updateIfChanged(id, value, updateFn) {
    if (prevValues[id] !== value) {
        prevValues[id] = value;
        updateFn(value);
    }

    // Periodically clean up stale entries (every 100 calls)
    prevValueCleanupCounter++;
    if (prevValueCleanupCounter >= 100) {
        prevValueCleanupCounter = 0;
        cleanupPrevValues();
    }
}

// Helper function to update text content only if changed
function updateTextIfChanged(id, text) {
    const key = `${id}_text`;
    if (prevValues[key] !== text) {
        prevValues[key] = text;
        document.getElementById(id).textContent = text;
    }
}

// Helper function to update innerHTML only if changed
function updateHtmlIfChanged(id, html) {
    const key = `${id}_html`;
    if (prevValues[key] !== html) {
        prevValues[key] = html;
        document.getElementById(id).innerHTML = html;
    }
}

// Helper function to update style only if changed
function updateStyleIfChanged(id, prop, value) {
    const key = `${id}_style_${prop}`;
    if (prevValues[key] !== value) {
        prevValues[key] = value;
        document.getElementById(id).style[prop] = value;
    }
}

// Time-travel state
let playbackMode = false; // false = live, true = historical playback
let isTimelineLoading = false; // Loading state for timeline data fetching
let currentTimestamp = null; // Current playback timestamp (seconds)
let firstTimestamp = null; // Earliest available data
let lastTimestamp = null; // Latest available data
const REWIND_STEP = 60; // 1 minute
let playbackInterval = null; // Auto-playback timer

// Playback buffer for efficient chunked loading
let playbackBuffer = {}; // Events grouped by second: { "123456": [events...], "123457": [events...] }
let bufferStart = null; // First second in buffer
let bufferEnd = null; // Last second in buffer
const BUFFER_SIZE = 60; // Fetch 60 seconds at a time
const PREFETCH_THRESHOLD = 50; // Prefetch when 50 seconds into current buffer
let lastPrefetchEnd = null; // Track last prefetched segment to avoid redundant fetches

// Fetch the most recent complete system state on load to initialize caches
async function fetchInitialState() {
    try {
        const resp = await fetch('/api/initial-state');
        const data = await resp.json();

        if(data.type === 'SystemMetrics') {
            // Populate caches with static/semi-static fields
            if(data.mem_total != null) cachedMemTotal = data.mem_total;
            if(data.swap_total != null) cachedSwapTotal = data.swap_total;
            if(data.disk_total != null) cachedDiskTotal = data.disk_total;
            if(data.net_ip != null) cachedNetIp = data.net_ip;
            if(data.net_gateway != null) cachedNetGateway = data.net_gateway;
            if(data.net_dns != null) cachedNetDns = data.net_dns;
            if(data.kernel != null) cachedKernel = data.kernel;
            if(data.cpu_model != null) cachedCpuModel = data.cpu_model;
            if(data.cpu_mhz != null) cachedCpuMhz = data.cpu_mhz;

            if(data.filesystems && data.filesystems.length > 0) {
                cachedFilesystems = data.filesystems;
                // Render filesystems immediately
                const filesystems = data.filesystems;
                filesystems.forEach((fs, i) => {
                    const pct = fs.total_bytes > 0 ? Math.round((fs.used_bytes / fs.total_bytes) * 100) : 0;
                    updateDiskBar(`disk_${i}`, pct, document.getElementById('diskContainer'), fs.mount_point, fs.used_bytes, fs.total_bytes);
                });
            }

            // Render network info immediately
            if(cachedNetIp) document.getElementById('netAddress').textContent = `Address: ${cachedNetIp}`;
            if(cachedNetGateway) document.getElementById('netGateway').textContent = `Gateway: ${cachedNetGateway}`;
            if(cachedNetDns) document.getElementById('netDns').textContent = `DNS: ${cachedNetDns}`;

            // Render kernel and CPU info immediately
            if(cachedKernel) document.getElementById('kernelRow').textContent = `Linux Kernel: ${cachedKernel}`;
            if(cachedCpuModel) document.getElementById('cpuDetailsRow').textContent = `CPU Details: ${cachedCpuModel}${cachedCpuMhz ? `, ${cachedCpuMhz}MHz` : ''}`;
        }
    } catch(e) {
        console.error('Failed to load initial state:', e);
    }
}

// Timeline visualization
let timelineData = null;
let timelineHoverX = null;  // Track mouse position for hover effect
let timelineHoverSetup = false;  // Prevent duplicate event listeners

async function fetchTimeline() {
    try {
        const resp = await fetch('/api/timeline');
        const data = await resp.json();
        timelineData = data;

        if(data.timeline && data.timeline.length > 0) {
            const canvas = document.getElementById('timelineChart');
            canvas.style.opacity = '1';

            if(!timelineHoverSetup) {
                setupTimelineHover();
                timelineHoverSetup = true;
            }

            drawTimeline();
        }
    } catch(e) {
        console.error('Failed to load timeline:', e);
    }
}

function setupTimelineHover() {
    const canvas = document.getElementById('timelineChart');

    canvas.addEventListener('mousemove', (e) => {
        const rect = canvas.getBoundingClientRect();
        timelineHoverX = e.clientX - rect.left;
        drawTimeline();
    });

    canvas.addEventListener('mouseleave', () => {
        timelineHoverX = null;
        drawTimeline();
    });

}

// Helper function to draw a smooth curve segment
function drawSegment(ctx, points) {
    if(points.length === 0) return;

    ctx.beginPath();
    ctx.moveTo(points[0].x, points[0].y);

    if(points.length === 1) {
        // Single point - just draw a small marker
        ctx.lineTo(points[0].x, points[0].y);
    } else if(points.length === 2) {
        // Two points - draw a line
        ctx.lineTo(points[1].x, points[1].y);
    } else {
        // Multiple points - use cubic Bezier curves for smooth interpolation
        for(let i = 0; i < points.length - 1; i++) {
            const curr = points[i];
            const next = points[i + 1];
            const prev = i > 0 ? points[i - 1] : curr;
            const after = i < points.length - 2 ? points[i + 2] : next;

            const cp1x = curr.x + (next.x - prev.x) / 6;
            const cp1y = curr.y + (next.y - prev.y) / 6;
            const cp2x = next.x - (after.x - curr.x) / 6;
            const cp2y = next.y - (after.y - curr.y) / 6;

            ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, next.x, next.y);
        }
    }
    ctx.stroke();
}

function drawTimeline() {
    if(!timelineData || !timelineData.timeline || timelineData.timeline.length === 0) return;

    const canvas = document.getElementById('timelineChart');
    const ctx = canvas.getContext('2d');

    // Set canvas internal dimensions to match display size (prevents stretching)
    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);

    // Use display dimensions for drawing
    const width = rect.width;
    const height = rect.height;

    // Clear canvas with transparent background
    ctx.clearRect(0, 0, width, height);

    const timeline = timelineData.timeline;
    const firstTs = timelineData.first_timestamp;
    const lastTs = timelineData.last_timestamp;
    const timeRange = lastTs - firstTs;

    if(timeRange <= 0 || timeline.length === 0) return;

    // Determine if we're hovering
    const isHovering = timelineHoverX !== null && timelineHoverX !== undefined;

    // Draw CPU usage line (blue) - with gap detection
    const cpuData = timeline
        .filter(p => p.cpu !== null && p.cpu !== undefined)
        .map(p => ({
            x: ((p.timestamp - firstTs) / timeRange) * width,
            y: height - ((p.cpu / 100) * (height - 8)) - 4,
            timestamp: p.timestamp
        }));

    if(cpuData.length > 0) {
        ctx.strokeStyle = isHovering ? 'rgba(59, 130, 246, 1)' : 'rgba(59, 130, 246, 0.5)'; // blue-500
        ctx.lineWidth = 1.5;

        // Draw with gap detection (break line when timestamps are more than 10 minutes apart)
        let segmentStart = 0;
        for(let i = 0; i < cpuData.length - 1; i++) {
            const timeDiff = cpuData[i + 1].timestamp - cpuData[i].timestamp;
            if(timeDiff > 600) { // Gap of more than 10 minutes
                // Draw segment from segmentStart to i
                drawSegment(ctx, cpuData.slice(segmentStart, i + 1));
                segmentStart = i + 1;
            }
        }
        // Draw final segment
        drawSegment(ctx, cpuData.slice(segmentStart));
    }

    // Draw Memory usage line (yellow) - with gap detection
    const memData = timeline
        .filter(p => p.mem !== null && p.mem !== undefined)
        .map(p => ({
            x: ((p.timestamp - firstTs) / timeRange) * width,
            y: height - ((p.mem / 100) * (height - 8)) - 4,
            timestamp: p.timestamp
        }));

    if(memData.length > 0) {
        ctx.strokeStyle = isHovering ? 'rgba(234, 179, 8, 1)' : 'rgba(234, 179, 8, 0.5)'; // yellow-500
        ctx.lineWidth = 1.5;

        // Draw with gap detection (break line when timestamps are more than 10 minutes apart)
        let segmentStart = 0;
        for(let i = 0; i < memData.length - 1; i++) {
            const timeDiff = memData[i + 1].timestamp - memData[i].timestamp;
            if(timeDiff > 600) { // Gap of more than 10 minutes
                // Draw segment from segmentStart to i
                drawSegment(ctx, memData.slice(segmentStart, i + 1));
                segmentStart = i + 1;
            }
        }
        // Draw final segment
        drawSegment(ctx, memData.slice(segmentStart));
    }

    // Find max count for scaling event count line
    const maxCount = Math.max(...timeline.map(p => p.count), 1);

    // Map event count data points to canvas coordinates
    const points = timeline.map(p => {
        const x = ((p.timestamp - firstTs) / timeRange) * width;
        const y = height - ((p.count / maxCount) * (height - 8)) - 4; // Leave 4px padding at top/bottom
        return { x, y, timestamp: p.timestamp };
    });

    // Draw event count curve using cubic Bezier curves
    ctx.beginPath();
    // gray-500/60: rgba(107, 114, 128, 0.6) - normal
    // gray-400: rgba(156, 163, 175, 1) - hover
    ctx.strokeStyle = isHovering ? 'rgba(107, 114, 128, 1)' : 'rgba(156, 163, 175, 0.8)';
    ctx.lineWidth = 1.5;

    if(points.length > 0) {
        ctx.moveTo(points[0].x, points[0].y);

        if(points.length === 2) {
            // Just draw a line for 2 points
            ctx.lineTo(points[1].x, points[1].y);
        } else if(points.length > 2) {
            // Use cubic Bezier curves for smooth interpolation
            for(let i = 0; i < points.length - 1; i++) {
                const curr = points[i];
                const next = points[i + 1];

                // Calculate control points for smooth curve
                // Use neighboring points to determine tangent direction
                const prev = i > 0 ? points[i - 1] : curr;
                const after = i < points.length - 2 ? points[i + 2] : next;

                const cp1x = curr.x + (next.x - prev.x) / 6;
                const cp1y = curr.y + (next.y - prev.y) / 6;
                const cp2x = next.x - (after.x - curr.x) / 6;
                const cp2y = next.y - (after.y - curr.y) / 6;

                ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, next.x, next.y);
            }
        }
    }

    ctx.stroke();

    // Draw vertical line at hover position
    if(isHovering && timelineHoverX >= 0 && timelineHoverX <= width) {
        ctx.beginPath();
        ctx.strokeStyle = 'rgba(156, 163, 175, 1)'; // gray-400 with transparency
        ctx.lineWidth = 1;
        ctx.moveTo(timelineHoverX, 0);
        ctx.lineTo(timelineHoverX, height);
        ctx.stroke();
    }

    // Draw vertical line for current playback position
    if(playbackMode && currentTimestamp) {
        const currentX = ((currentTimestamp - firstTs) / timeRange) * width;
        if(currentX >= 0 && currentX <= width) {
            ctx.beginPath();
            ctx.strokeStyle = 'rgba(59, 130, 246, 0.8)'; // blue-500
            ctx.lineWidth = 1.5;
            ctx.moveTo(currentX, 0);
            ctx.lineTo(currentX, height);
            ctx.stroke();
        }
    }
}

// Handle timeline click to jump to timestamp
document.getElementById('timelineChart').addEventListener('click', (e) => {
    if(!timelineData || !timelineData.timeline || timelineData.timeline.length === 0) return;

    // Show loading spinner
    showTimelineLoader();

    const canvas = document.getElementById('timelineChart');
    const rect = canvas.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const width = rect.width;

    const firstTs = timelineData.first_timestamp;
    const lastTs = timelineData.last_timestamp;
    const timeRange = lastTs - firstTs;

    // Calculate timestamp from click position
    const clickRatio = clickX / width;
    const targetTimestamp = firstTs + (clickRatio * timeRange);

    // Stop any auto-playback
    if(playbackInterval) {
        clearTimeout(playbackInterval);
        playbackInterval = null;
    }

    // Update button states to show paused
    isPaused = true;
    document.getElementById('pauseBtn').style.display = 'none';
    document.getElementById('playBtn').style.display = 'block';
    syncHeaderButtons();

    // Jump to the timestamp
    jumpToTimestamp(Math.floor(targetTimestamp));
});

// Handle timeline hover to show timestamp
document.getElementById('timelineChart').addEventListener('mousemove', (e) => {
    if(!timelineData || !timelineData.timeline || timelineData.timeline.length === 0) return;

    const canvas = document.getElementById('timelineChart');
    const rect = canvas.getBoundingClientRect();
    const hoverX = e.clientX - rect.left;
    const width = rect.width;

    const firstTs = timelineData.first_timestamp;
    const lastTs = timelineData.last_timestamp;
    const timeRange = lastTs - firstTs;

    const hoverRatio = hoverX / width;
    const hoverTimestamp = firstTs + (hoverRatio * timeRange);

    const date = new Date(hoverTimestamp * 1000);
    const now = new Date();

    // Check if the date is today
    const isToday = date.getFullYear() === now.getFullYear() &&
                    date.getMonth() === now.getMonth() &&
                    date.getDate() === now.getDate();

    // If today, show just the time; otherwise show full date
    const displayText = isToday
        ? date.toTimeString().substring(0, 8)
        : formatDate(date);

    // Find closest timeline data point
    let closestPoint = null;
    let minDiff = Infinity;
    for(const point of timelineData.timeline) {
        const diff = Math.abs(point.timestamp - hoverTimestamp);
        if(diff < minDiff) {
            minDiff = diff;
            closestPoint = point;
        }
    }

    // Build tooltip with metrics
    let tooltip = `Jump to ${displayText}`;
    if(closestPoint) {
        const metrics = [];
        if(closestPoint.count !== null && closestPoint.count !== undefined) {
            metrics.push(`Events: ${closestPoint.count}`);
        }
        if(closestPoint.cpu !== null && closestPoint.cpu !== undefined) {
            metrics.push(`CPU: ${closestPoint.cpu.toFixed(1)}%`);
        }
        if(closestPoint.mem !== null && closestPoint.mem !== undefined) {
            metrics.push(`Memory: ${closestPoint.mem.toFixed(1)}%`);
        }
        if(metrics.length > 0) {
            tooltip += '\n' + metrics.join(', ');
        }
    }

    canvas.title = tooltip;
});

// Fetch available time range on load
async function fetchPlaybackInfo() {
    try {
        const resp = await fetch('/api/playback/info');
        const data = await resp.json();
        firstTimestamp = data.first_timestamp;
        lastTimestamp = data.last_timestamp;

        if(firstTimestamp && lastTimestamp) {
            const duration = lastTimestamp - firstTimestamp;
            const hours = Math.floor(duration / 3600);
            const mins = Math.floor((duration % 3600) / 60);

            // Show when the data is from
            const lastDate = new Date(lastTimestamp * 1000);
            const ageSeconds = Math.floor(Date.now() / 1000) - lastTimestamp;
            const ageHours = Math.floor(ageSeconds / 3600);
            const ageMins = Math.floor((ageSeconds % 3600) / 60);

            document.getElementById('timeRange').textContent =
                `${hours}h ${mins}m`;
        }
    } catch(e) {
        console.error('Failed to fetch playback info:', e);
    }
}

// Fetch and populate playback buffer with events
async function fetchPlaybackBuffer(startTimestamp, endTimestamp) {
    try {
        const url = `/api/playback/events?start=${startTimestamp}&end=${endTimestamp}&limit=2000`;
        const resp = await fetch(url);
        const data = await resp.json();

        // Group events by second (rounded timestamp)
        const buffer = {};
        if (data.events) {
            data.events.forEach(event => {
                const second = Math.floor(event.timestamp / 1000); // Convert ms to seconds
                if (!buffer[second]) {
                    buffer[second] = [];
                }
                buffer[second].push(event);
            });
        }

        // Store metadata if present
        if (data.metadata) {
            if(data.metadata.mem_total_bytes) cachedMemTotal = data.metadata.mem_total_bytes;
            if(data.metadata.swap_total_bytes) cachedSwapTotal = data.metadata.swap_total_bytes;
            if(data.metadata.disk_total_bytes) cachedDiskTotal = data.metadata.disk_total_bytes;
            if(data.metadata.filesystems && data.metadata.filesystems.length > 0) cachedFilesystems = data.metadata.filesystems;
            if(data.metadata.net_ip) cachedNetIp = data.metadata.net_ip;
            if(data.metadata.net_gateway) cachedNetGateway = data.metadata.net_gateway;
            if(data.metadata.net_dns) cachedNetDns = data.metadata.net_dns;
            if(data.metadata.kernel_version) cachedKernel = data.metadata.kernel_version;
            if(data.metadata.cpu_model) cachedCpuModel = data.metadata.cpu_model;
            if(data.metadata.cpu_mhz) cachedCpuMhz = data.metadata.cpu_mhz;
        }

        return buffer;
    } catch(e) {
        console.error('Failed to fetch playback buffer:', e);
        return {};
    }
}

// Process events for a specific second from the playback buffer
function processSecondFromBuffer(timestamp) {
    const events = playbackBuffer[timestamp] || [];

    let latestSystemMetrics = null;
    let latestProcessSnapshot = null;

    events.forEach(event => {
        if(event.type === 'SystemMetrics') {
            latestSystemMetrics = event;
            // Add to history
            cpuHistory.push(event.cpu || 0);
            memoryHistory.push(event.mem || 0);
            netDownHistory.push(event.net_recv || 0);
            netUpHistory.push(event.net_send || 0);
            if(cpuHistory.length > MAX_HISTORY) cpuHistory.shift();
            if(memoryHistory.length > MAX_HISTORY) memoryHistory.shift();
            if(netDownHistory.length > MAX_HISTORY) netDownHistory.shift();
            if(netUpHistory.length > MAX_HISTORY) netUpHistory.shift();
        } else if(event.type === 'ProcessSnapshot') {
            latestProcessSnapshot = event;
        } else {
            addEventToLog(event);
        }
    });

    // Render latest state
    if(latestSystemMetrics) {
        lastStats = latestSystemMetrics;
        render();
    }
    if(latestProcessSnapshot) {
        updateProcs(latestProcessSnapshot);
    }

    drawTimeline();
}

// Show/hide timeline loading spinner
function showTimelineLoader() {
    isTimelineLoading = true;
    const spinner = document.getElementById('timelineLoadingSpinner');
    if (spinner) spinner.style.display = 'inline-block';
}

function hideTimelineLoader() {
    isTimelineLoading = false;
    const spinner = document.getElementById('timelineLoadingSpinner');
    if (spinner) spinner.style.display = 'none';
}

// Jump to a specific timestamp and load data
// Now uses chunked buffering for efficient playback
async function jumpToTimestamp(timestamp, incremental = false) {
    if(!timestamp) return;

    // Ensure spinner is visible (in case called directly)
    if (!incremental) {
        showTimelineLoader();
    }

    currentTimestamp = timestamp;
    playbackMode = true;

    // Update time display - add visual indicator for playback mode
    const dt = new Date(timestamp * 1000);
    document.getElementById('timeDisplay').textContent =
        '⏱ ' + dt.toLocaleTimeString();
    document.getElementById('timeDisplay').style.color = '#f59e0b'; // amber color

    // Update playback time display
    document.getElementById('playbackTimeDisplay').style.display = 'flex';
    const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
    const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
    const formatted = `${days[dt.getDay()]}, ${dt.getDate()} ${months[dt.getMonth()]} ${dt.getFullYear()}, ${dt.toLocaleTimeString()}`;
    document.getElementById('playbackTime').textContent = '⏱ ' + formatted;

    // Check if timestamp is in current buffer
    const inBuffer = bufferStart && bufferEnd && timestamp >= bufferStart && timestamp <= bufferEnd;

    if(incremental && inBuffer) {
        // Just process this second from the buffer (already loaded)
        processSecondFromBuffer(timestamp);

        // Prefetch next chunk if approaching end of buffer (only once per segment)
        if(timestamp >= bufferStart + PREFETCH_THRESHOLD && timestamp < bufferEnd) {
            const nextSegmentEnd = bufferEnd + BUFFER_SIZE;
            // Only prefetch if we haven't already fetched this segment
            if(lastPrefetchEnd !== nextSegmentEnd) {
                const nextBuffer = await fetchPlaybackBuffer(bufferEnd + 1, nextSegmentEnd);
                // Merge into existing buffer
                Object.assign(playbackBuffer, nextBuffer);
                bufferEnd = nextSegmentEnd;
                lastPrefetchEnd = nextSegmentEnd;
            }
        }
        return;
    }

    // Full jump - clear everything and reload
    cpuHistory.length = 0;
    memoryHistory.length = 0;
    netDownHistory.length = 0;
    netUpHistory.length = 0;
    Object.keys(diskIoHistoryMap).forEach(k => delete diskIoHistoryMap[k]);

    // Clear event buffer, keys, and container
    eventBuffer.length = 0;
    eventKeys.clear();
    document.getElementById('eventsContainer').innerHTML = '';

    // Clean up prevValues cache to prevent memory leak
    cleanupPrevValues();

    // Fetch past 60 seconds for chart history using the count API
    try {
        const historyUrl = `/api/playback/events?timestamp=${timestamp}&count=60`;
        const historyResp = await fetch(historyUrl);
        const historyData = await historyResp.json();


        if(historyData.events && historyData.events.length > 0) {
            const timeDisplay = document.getElementById('timeDisplay');
            timeDisplay.title = 'Click to select time, Shift+Click to go Live';

            // Build chart history from past events and populate event log
            historyData.events.forEach(event => {
                if(event.type === 'SystemMetrics') {
                    cpuHistory.push(event.cpu || 0);
                    memoryHistory.push(event.mem || 0);
                    netDownHistory.push(event.net_recv || 0);
                    netUpHistory.push(event.net_send || 0);
                } else if(event.type !== 'ProcessSnapshot') {
                    // Add non-SystemMetrics, non-ProcessSnapshot events to the log
                    addEventToLog(event);
                }
            });

            // Trim to MAX_HISTORY
            if(cpuHistory.length > MAX_HISTORY) {
                cpuHistory.splice(0, cpuHistory.length - MAX_HISTORY);
                memoryHistory.splice(0, memoryHistory.length - MAX_HISTORY);
                netDownHistory.splice(0, netDownHistory.length - MAX_HISTORY);
                netUpHistory.splice(0, netUpHistory.length - MAX_HISTORY);
            }

            // Handle metadata
            if(historyData.metadata) {
                if(historyData.metadata.mem_total_bytes) cachedMemTotal = historyData.metadata.mem_total_bytes;
                if(historyData.metadata.swap_total_bytes) cachedSwapTotal = historyData.metadata.swap_total_bytes;
                if(historyData.metadata.disk_total_bytes) cachedDiskTotal = historyData.metadata.disk_total_bytes;
                if(historyData.metadata.filesystems && historyData.metadata.filesystems.length > 0) cachedFilesystems = historyData.metadata.filesystems;
                if(historyData.metadata.net_ip) cachedNetIp = historyData.metadata.net_ip;
                if(historyData.metadata.net_gateway) cachedNetGateway = historyData.metadata.net_gateway;
                if(historyData.metadata.net_dns) cachedNetDns = historyData.metadata.net_dns;
                if(historyData.metadata.kernel_version) cachedKernel = historyData.metadata.kernel_version;
                if(historyData.metadata.cpu_model) cachedCpuModel = historyData.metadata.cpu_model;
                if(historyData.metadata.cpu_mhz) cachedCpuMhz = historyData.metadata.cpu_mhz;
            }
        }
    } catch(e) {
        console.error('Failed to load history:', e);
        // Hide spinner on error
        hideTimelineLoader();
    }

    // Fetch forward buffer for playback (60 seconds ahead)
    bufferStart = timestamp;
    bufferEnd = timestamp + BUFFER_SIZE;
    playbackBuffer = await fetchPlaybackBuffer(bufferStart, bufferEnd);
    lastPrefetchEnd = null; // Reset prefetch tracker for new buffer

    // Process current second from buffer
    processSecondFromBuffer(timestamp);

    // Update timeline visualization
    drawTimeline();

    // Hide loading spinner
    hideTimelineLoader();
}

// Rewind button
document.getElementById('rewindBtn').addEventListener('click', doRewind);

// Fast-forward button
document.getElementById('fastForwardBtn').addEventListener('click', doFastForward);

// Pause button
document.getElementById('pauseBtn').addEventListener('click', doPause);

// Shared play logic
async function doPlay() {
    if(playbackMode && currentTimestamp) {
        // Resume playback: auto-advance through history
        isPaused = false;
        document.getElementById('playBtn').style.display = 'none';
        document.getElementById('pauseBtn').style.display = 'block';
        syncHeaderButtons();

        // Calculate a reasonable "live" threshold - within 10 seconds of now
        const liveThreshold = Math.floor(Date.now() / 1000) - 10;

        // Auto-advance recursively (waits for each fetch to complete)
        const autoAdvance = async () => {
            // Check if still in playback mode
            if(!playbackMode) {
                return;
            }

            if(currentTimestamp >= liveThreshold) {
                // Reached live time, switch to live mode
                goLive();
            } else {
                const nextTimestamp = currentTimestamp + 1;
                await jumpToTimestamp(nextTimestamp, true);  // incremental=true

                // Schedule next tick
                playbackInterval = setTimeout(autoAdvance, 1000);
            }
        };

        // Start first advance immediately
        await autoAdvance();
    } else {
        // Not in playback mode, just unpause
        goLive();
    }
}

// Play button - either resume playback or return to live
document.getElementById('playBtn').addEventListener('click', doPlay);

// Return to live mode
function goLive() {
    isPaused = false;
    playbackMode = false;
    currentTimestamp = null;

    if(playbackInterval) {
        clearTimeout(playbackInterval);
        playbackInterval = null;
    }

    document.getElementById('playBtn').style.display = 'none';
    document.getElementById('pauseBtn').style.display = 'block';

    // Hide playback time display when going live
    document.getElementById('playbackTimeDisplay').style.display = 'none';

    // Show "Live" or "Disconnected" based on connection status
    const isConnected = ws && ws.readyState === 1;
    const timeDisplay = document.getElementById('timeDisplay');
    timeDisplay.textContent = isConnected ? 'Live' : 'Disconnected';
    timeDisplay.style.color = isConnected ? '#6b7280' : '#ef4444'; // gray-500 or red-500
    timeDisplay.title = 'Click to select time, Shift+Click to go Live';

    // Clear history buffers so they rebuild from live data
    cpuHistory.length = 0;
    memoryHistory.length = 0;
    netDownHistory.length = 0;
    netUpHistory.length = 0;
    Object.keys(diskIoHistoryMap).forEach(k => delete diskIoHistoryMap[k]);

    // Clear event buffer, keys, and container
    eventBuffer.length = 0;
    eventKeys.clear();
    document.getElementById('eventsContainer').innerHTML = '';

    // Clean up prevValues cache to prevent memory leak
    cleanupPrevValues();

    // Update timeline visualization (clears vertical line)
    drawTimeline();

    // Update header controls visibility
    updateConnectionStatus();
}

// Sync header play/pause buttons with main buttons
function syncHeaderButtons() {
    const mainPauseVisible = document.getElementById('pauseBtn').style.display !== 'none';
    document.getElementById('headerPauseBtn').style.display = mainPauseVisible ? 'inline' : 'none';
    document.getElementById('headerPlayBtn').style.display = mainPauseVisible ? 'none' : 'inline';
}

// Shared rewind logic
function doRewind() {
    // Show loading spinner
    showTimelineLoader();

    if(playbackInterval) {
        clearTimeout(playbackInterval);
        playbackInterval = null;
    }
    if(!playbackMode) {
        const now = Math.floor(Date.now() / 1000);
        jumpToTimestamp(now - REWIND_STEP);
        isPaused = true;
        document.getElementById('pauseBtn').style.display = 'none';
        document.getElementById('playBtn').style.display = 'block';
        syncHeaderButtons();
    } else {
        const newTime = Math.max(firstTimestamp || 0, currentTimestamp - REWIND_STEP);
        jumpToTimestamp(newTime);
        isPaused = true;
        document.getElementById('pauseBtn').style.display = 'none';
        document.getElementById('playBtn').style.display = 'block';
        syncHeaderButtons();
    }
}

// Shared fast-forward logic
function doFastForward() {
    if(!playbackMode) return;

    // Show loading spinner
    showTimelineLoader();

    if(playbackInterval) {
        clearTimeout(playbackInterval);
        playbackInterval = null;
    }
    const target = currentTimestamp + REWIND_STEP;
    const maxTime = lastTimestamp || Math.floor(Date.now() / 1000);
    const newTime = Math.min(target, maxTime);
    jumpToTimestamp(newTime);
    isPaused = true;
    document.getElementById('pauseBtn').style.display = 'none';
    document.getElementById('playBtn').style.display = 'block';
    syncHeaderButtons();
}

// Shared pause logic
function doPause() {
    isPaused = true;
    document.getElementById('pauseBtn').style.display = 'none';
    document.getElementById('playBtn').style.display = 'block';
    syncHeaderButtons();
    if(playbackInterval) {
        clearTimeout(playbackInterval);
        playbackInterval = null;
    }
    if(!playbackMode) {
        const now = Math.floor(Date.now() / 1000);
        currentTimestamp = now;
        playbackMode = true;
        // Update playback time display when pausing from live mode
        const dt = new Date(now * 1000);
        document.getElementById('playbackTimeDisplay').style.display = 'flex';
        const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
        const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
        const formatted = `${days[dt.getDay()]}, ${dt.getDate()} ${months[dt.getMonth()]} ${dt.getFullYear()}, ${dt.toLocaleTimeString()}`;
        document.getElementById('playbackTime').textContent = '⏱ ' + formatted;
    }
}

// Header button handlers
document.getElementById('headerRewindBtn').addEventListener('click', doRewind);
document.getElementById('headerFastForwardBtn').addEventListener('click', doFastForward);
document.getElementById('headerPauseBtn').addEventListener('click', doPause);
document.getElementById('headerPlayBtn').addEventListener('click', doPlay);

// Time display click - either go live or open picker
document.getElementById('timeDisplay').addEventListener('click', (e) => {
    if(e.shiftKey && playbackMode) {
        // Shift+click: Go live
        goLive();
        return;
    }

    const picker = document.getElementById('timePicker');

    if(firstTimestamp && lastTimestamp) {
        // Set picker range
        const firstDate = new Date(firstTimestamp * 1000);
        const lastDate = new Date(lastTimestamp * 1000);

        picker.min = firstDate.toISOString().slice(0, 16);
        picker.max = lastDate.toISOString().slice(0, 16);

        // Set current value
        const current = currentTimestamp || Math.floor(Date.now() / 1000);
        picker.value = new Date(current * 1000).toISOString().slice(0, 16);

        picker.style.display = 'block';
        picker.focus();
    }
});

document.getElementById('timePicker').addEventListener('change', (e) => {
    const selectedDate = new Date(e.target.value);
    const timestamp = Math.floor(selectedDate.getTime() / 1000);

    jumpToTimestamp(timestamp);
    e.target.style.display = 'none';

    // Enable pause mode
    isPaused = true;
    document.getElementById('pauseBtn').style.display = 'none';
    document.getElementById('playBtn').style.display = 'block';
    syncHeaderButtons();
});

document.getElementById('timePicker').addEventListener('blur', (e) => {
    setTimeout(() => e.target.style.display = 'none', 200);
});

// Fetch playback info and timeline on startup
// Initial state is sent via WebSocket on connection
fetchPlaybackInfo();
fetchTimeline();

const fmt = b => {
    if(!b) return '0B';
    const k=1024, s=['B','KB','MB','GB','TB'], i=Math.floor(Math.log(b)/Math.log(k));
    return (b/Math.pow(k,i)).toFixed(i>1?1:0)+s[i];
};
const fmtRate = b => fmt(b)+'/s';
const formatUptime = s => {
    const d=Math.floor(s/86400),h=Math.floor((s%86400)/3600),m=Math.floor((s%3600)/60),sec=Math.floor(s%60);
    return d>0?`${d}d ${h}h ${m}m`:h>0?`${h}h ${m}m ${sec}s`:`${m}m ${sec}s`;
};
const formatDate = date => {
    const days=['Sun','Mon','Tue','Wed','Thu','Fri','Sat'], mons=['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
    return `${days[date.getDay()]}, ${String(date.getDate()).padStart(2,'0')} ${mons[date.getMonth()]} ${date.getFullYear()}, ${date.toTimeString().substring(0,8)}`;
};

function updateBar(id, pct, container, labelText, rightLabel){
    let el = document.getElementById(id);
    if(!el){
        container.insertAdjacentHTML('beforeend', `<div class="text-gray-500 flex items-center justify-between" id="row_${id}">
            <span id="lbl_${id}">${labelText}</span>
            <span class="flex items-center">
                <span id="rlbl_${id}" class="${rightLabel ? '' : 'hidden'}">${rightLabel || ''}</span>
                <span class="inline-block w-32 h-3 bg-gray-200 overflow-hidden align-middle ml-1" style="border-radius:1px">
                    <span id="${id}" class="block h-full transition-all duration-300" style="width:0%;border-radius:1px"></span>
                </span>
            </span>
        </div>`);
        el = document.getElementById(id);
    }
    const color = pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-yellow-500' : 'bg-green-500';
    el.style.width = Math.min(100, pct) + '%';
    el.className = `block h-full transition-all duration-300 ${color}`;
    el.style.borderRadius = '1px';
    const lbl = document.getElementById('lbl_'+id);
    if(lbl) lbl.textContent = labelText;
    const rlbl = document.getElementById('rlbl_'+id);
    if(rlbl && rightLabel !== undefined) { rlbl.textContent = rightLabel; rlbl.className = ''; }
}

function updateCoreBar(id, pct, container, coreNum){
    let el = document.getElementById(id);
    if(!el){
        container.insertAdjacentHTML('beforeend', `<div class="text-gray-500 flex items-center gap-4" id="row_${id}" title="CPU usage for core ${coreNum}">
            <span class="w-10">CPU${coreNum}</span>
            <span class="relative flex-1 bg-gray-200" style="height:10px;border-radius:1px">
                <span id="${id}" class="block h-full transition-all duration-300" style="width:0%;border-radius:1px"></span>
                <span id="pct_${id}" class="absolute inset-0 flex items-center justify-center text-gray-500/60 overflow-visible"></span>
            </span>
        </div>`);
        el = document.getElementById(id);
    }
    const color = pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-yellow-500' : 'bg-green-500';
    const widthValue = Math.min(100, pct) + '%';
    updateStyleIfChanged(id, 'width', widthValue);
    updateIfChanged(`${id}_class`, color, () => {
        el.className = `block h-full transition-all duration-300 ${color}`;
    });
    updateTextIfChanged(`pct_${id}`, pct.toFixed(1) + '%');
}

function updateRamBar(pct, used, container){
    let el = document.getElementById('ramBar');
    if(!el){
        container.innerHTML = `<div class="text-gray-500 flex items-center gap-4">
            <span id="ramLabel">RAM Used: ${fmt(used)}</span>
            <span class="relative flex-1 bg-gray-200" style="height:10px;border-radius:1px">
                <span id="ramBar" class="block h-full transition-all duration-300" style="width:0%;border-radius:1px"></span>
                <span id="ramPct" class="absolute inset-0 flex items-center justify-center text-gray-500/60 overflow-visible"></span>
            </span>
        </div>`;
        el = document.getElementById('ramBar');
    }
    const color = pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-yellow-500' : 'bg-green-500';
    const widthValue = Math.min(100, pct) + '%';
    updateStyleIfChanged('ramBar', 'width', widthValue);
    updateIfChanged('ramBar_class', color, () => {
        el.className = `block h-full transition-all duration-300 ${color}`;
    });
    updateTextIfChanged('ramLabel', `RAM Used: ${fmt(used)}`);
    updateTextIfChanged('ramPct', pct.toFixed(1) + '%');
}

function getUsageColor(pct){
    // Discrete Tailwind colors based on usage thresholds
    if(pct >= 90) return 'rgb(239, 68, 68)';    // red-500
    if(pct >= 80) return 'rgb(248, 113, 113)';  // red-400
    if(pct >= 70) return 'rgb(252, 165, 165)';  // red-300
    if(pct >= 60) return 'rgb(234, 179, 8)';    // yellow-500
    if(pct >= 50) return 'rgb(250, 204, 21)';   // yellow-400
    if(pct >= 40) return 'rgb(253, 224, 71)';   // yellow-300
    if(pct >= 30) return 'rgb(163, 230, 53)';   // lime-400
    if(pct >= 20) return 'rgb(132, 204, 22)';   // lime-500
    if(pct >= 10) return 'rgb(34, 197, 94)';    // green-500
    return 'rgb(74, 222, 128)';                  // green-400
}

function drawChart(canvasId, history){
    const canvas = document.getElementById(canvasId);
    if (!canvas) return;

    // Use cached context or create new one
    let ctx = canvasContextCache[canvasId];
    if (!ctx) {
        ctx = canvas.getContext('2d', { alpha: false }); // alpha: false for better performance
        canvasContextCache[canvasId] = ctx;
    }

    const dpr = window.devicePixelRatio || 1;

    // Set canvas size accounting for device pixel ratio (only if changed)
    const rect = canvas.getBoundingClientRect();
    const newWidth = rect.width * dpr;
    const newHeight = rect.height * dpr;

    if (canvas.width !== newWidth || canvas.height !== newHeight) {
        canvas.width = newWidth;
        canvas.height = newHeight;
        ctx.scale(dpr, dpr);
    }

    const width = rect.width;
    const height = rect.height;
    const barWidth = width / MAX_HISTORY;

    // Clear canvas and set background to gray-50
    canvas.width = canvas.width;
    ctx.scale(dpr, dpr);
    ctx.fillStyle = '#f9fafb'; // gray-50
    ctx.fillRect(0, 0, width, height);

    // Batch fillRect calls by color to reduce state changes
    const barsByColor = {};
    history.forEach((pct, i) => {
        const x = (MAX_HISTORY - history.length + i) * barWidth;
        const barHeight = (pct / 100) * height;
        const y = height - barHeight;
        const color = getUsageColor(pct);

        if (!barsByColor[color]) barsByColor[color] = [];
        barsByColor[color].push({x, y, barWidth, barHeight});
    });

    // Draw all bars of the same color together
    Object.keys(barsByColor).forEach(color => {
        ctx.fillStyle = color;
        barsByColor[color].forEach(bar => {
            ctx.fillRect(bar.x, bar.y, bar.barWidth, bar.barHeight);
        });
    });
}

function drawNetworkChart(canvasId, history){
    const canvas = document.getElementById(canvasId);
    if (!canvas) return;

    // Use cached context or create new one
    let ctx = canvasContextCache[canvasId];
    if (!ctx) {
        ctx = canvas.getContext('2d', { alpha: false }); // alpha: false for better performance
        canvasContextCache[canvasId] = ctx;
    }

    const dpr = window.devicePixelRatio || 1;

    // Set canvas size accounting for device pixel ratio (only if changed)
    const rect = canvas.getBoundingClientRect();
    const newWidth = rect.width * dpr;
    const newHeight = rect.height * dpr;

    if (canvas.width !== newWidth || canvas.height !== newHeight) {
        canvas.width = newWidth;
        canvas.height = newHeight;
        ctx.scale(dpr, dpr);
    }

    const width = rect.width;
    const height = rect.height;
    const barWidth = width / MAX_HISTORY;

    // Clear canvas and set background to gray-50
    canvas.width = canvas.width;
    ctx.scale(dpr, dpr);
    ctx.fillStyle = '#f9fafb'; // gray-50
    ctx.fillRect(0, 0, width, height);

    // Find max value for scaling
    const maxVal = Math.max(...history, 1); // At least 1 to avoid division by zero

    // Batch fillRect calls by color to reduce state changes
    const barsByColor = {};
    history.forEach((val, i) => {
        const x = (MAX_HISTORY - history.length + i) * barWidth;
        const pct = (val / maxVal) * 100;
        const barHeight = (val / maxVal) * height;
        const y = height - barHeight;
        const color = getUsageColor(pct);

        if (!barsByColor[color]) barsByColor[color] = [];
        barsByColor[color].push({x, y, barWidth, barHeight});
    });

    // Draw all bars of the same color together
    Object.keys(barsByColor).forEach(color => {
        ctx.fillStyle = color;
        barsByColor[color].forEach(bar => {
            ctx.fillRect(bar.x, bar.y, bar.barWidth, bar.barHeight);
        });
    });
}

function updateMemoryChart(){
    drawChart('memoryChart', memoryHistory);
}

function updateCpuChart(){
    drawChart('cpuChart', cpuHistory);
}

function updateNetDownChart(){
    drawNetworkChart('netDownChart', netDownHistory);
}

function updateNetUpChart(){
    drawNetworkChart('netUpChart', netUpHistory);
}

function updateDiskBar(id, pct, container, mount, used, total){
    let el = document.getElementById(id);
    if(!el){
        container.insertAdjacentHTML('beforeend', `<div class="text-gray-500 flex items-center gap-4" id="row_${id}">
            <span id="lbl_${id}" class="flex-1">${mount}</span>
            <span><span id="used_${id}" class="text-gray-400">${fmt(used)}</span>/<span id="total_${id}">${fmt(total)}</span></span>
            <span class="relative bg-gray-200" style="height:10px;width:128px;border-radius:1px">
                <span id="${id}" class="block h-full transition-all duration-300" style="width:0%;border-radius:1px"></span>
                <span id="pct_${id}" class="absolute inset-0 flex items-center justify-center text-gray-500/60 overflow-visible"></span>
            </span>
        </div>`);
        el = document.getElementById(id);
    }
    const color = pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-yellow-500' : 'bg-green-500';
    const widthValue = Math.min(100, pct) + '%';
    updateStyleIfChanged(id, 'width', widthValue);
    updateIfChanged(`${id}_class`, color, () => {
        el.className = `block h-full transition-all duration-300 ${color}`;
    });
    updateTextIfChanged(`lbl_${id}`, mount);
    updateTextIfChanged(`pct_${id}`, pct + '%');
    updateTextIfChanged(`used_${id}`, fmt(used));
    updateTextIfChanged(`total_${id}`, fmt(total));
}

function updateDiskIo(disks){
    const section = document.getElementById('diskIoSection');
    const table = document.getElementById('diskIoTable');
    const tbody = document.getElementById('diskIoTableBody');

    if(!disks || disks.length === 0){
        updateStyleIfChanged('diskIoSection', 'display', 'none');
        updateStyleIfChanged('diskIoTable', 'display', 'none');
        if(prevValues['diskIoTableBody_cleared'] !== true) {
            prevValues['diskIoTableBody_cleared'] = true;
            tbody.innerHTML = '';
        }
        return;
    }

    updateStyleIfChanged('diskIoSection', 'display', 'flex');
    updateStyleIfChanged('diskIoTable', 'display', 'table');
    prevValues['diskIoTableBody_cleared'] = false;

    // Update or create rows for each disk
    disks.forEach((disk, i) => {
        const deviceKey = disk.device;

        // Initialize history for this disk if needed
        if(!diskIoHistoryMap[deviceKey]){
            diskIoHistoryMap[deviceKey] = [];
        }

        // Store raw throughput bytes for dynamic scaling
        const totalThroughput = disk.read + disk.write;

        // Add to history
        diskIoHistoryMap[deviceKey].push(totalThroughput);
        if(diskIoHistoryMap[deviceKey].length > MAX_HISTORY){
            diskIoHistoryMap[deviceKey].shift();
        }

        // Check if row exists
        let row = document.getElementById(`diskio_row_${i}`);
        if(!row){
            const tr = document.createElement('tr');
            tr.id = `diskio_row_${i}`;
            const tempText = disk.temp ? disk.temp.toFixed(0) + '°C' : '--';
            tr.innerHTML = `
                <td style="width:60px">${disk.device}</td>
                <td class="text-right" style="width:80px"><span id="diskio_read_${i}">${fmt(disk.read)}/s</span></td>
                <td class="text-right" style="width:80px"><span id="diskio_write_${i}">${fmt(disk.write)}/s</span></td>
                <td class="text-right text-gray-400" style="width:50px"><span id="diskio_temp_${i}">${tempText}</span></td>
                <td style="width:128px;text-align:right;vertical-align:middle"><canvas id="diskio_chart_${i}" style="height:10px;width:128px;" class="ml-auto"></canvas></td>
            `;
            tbody.appendChild(tr);
        } else {
            // Update existing row (only if changed)
            const readText = fmt(disk.read) + '/s';
            const writeText = fmt(disk.write) + '/s';
            const tempText = disk.temp ? disk.temp.toFixed(0) + '°C' : '--';
            updateTextIfChanged(`diskio_read_${i}`, readText);
            updateTextIfChanged(`diskio_write_${i}`, writeText);
            updateTextIfChanged(`diskio_temp_${i}`, tempText);
        }

        // Draw chart for this disk (use dynamic scaling like network charts)
        drawNetworkChart(`diskio_chart_${i}`, diskIoHistoryMap[deviceKey]);
    });
}

// Cache for process table rows to avoid recreating DOM elements
const procRowCache = {};

function updateProcTable(tableId, procs, memTotal){
    const tbody = document.getElementById(tableId);
    if (!tbody) return;

    // Build new rows efficiently
    const fragment = document.createDocumentFragment();
    const newRows = [];

    procs.forEach((p, i) => {
        const memPct = memTotal > 0 ? (p.mem_bytes / memTotal) * 100 : 0;
        const rowId = `${tableId}_${p.pid}`;

        // Check if we can reuse an existing row
        let tr = procRowCache[rowId];
        if (!tr) {
            tr = document.createElement('tr');
            tr.id = rowId;
            procRowCache[rowId] = tr;
        }

        // Only update if data changed (check using cache)
        const rowData = `${p.name}|${p.user}|${p.pid}|${p.cpu_percent.toFixed(1)}|${memPct.toFixed(1)}`;
        if (prevValues[`${rowId}_data`] !== rowData) {
            prevValues[`${rowId}_data`] = rowData;
            tr.innerHTML = `<td>${p.name}</td><td class="pr-2">${p.user || '-'}</td><td>${p.pid}</td><td class="text-right">${p.cpu_percent.toFixed(1)}%</td><td class="text-right">${memPct.toFixed(1)}%</td>`;
        }

        fragment.appendChild(tr);
        newRows.push(rowId);
    });

    // Replace table contents efficiently
    tbody.innerHTML = '';
    tbody.appendChild(fragment);

    // Clean up cache for rows no longer in use
    Object.keys(procRowCache).forEach(key => {
        if (key.startsWith(tableId + '_') && !newRows.includes(key)) {
            delete procRowCache[key];
            delete prevValues[`${key}_data`];
        }
    });
}

function render(){
    if(!lastStats)return;
    const e=lastStats;

    // Show content on first data load
    const mainContent = document.getElementById('mainContent');
    if(mainContent.style.display === 'none'){
        mainContent.style.display = 'block';
    }

    // Always show the timestamp from the event data (whether live or historical)
    if(e.timestamp) {
        const eventDate = new Date(e.timestamp);
        if(!isNaN(eventDate.getTime())) {
            updateTextIfChanged('datetime', formatDate(eventDate));
        } else {
            updateTextIfChanged('datetime', formatDate(new Date()));
        }
    } else {
        updateTextIfChanged('datetime', formatDate(new Date()));
    }
    const uptimeText = e.system_uptime_seconds ? `Uptime: ${formatUptime(e.system_uptime_seconds)}` : '';
    updateTextIfChanged('uptime', uptimeText);
    updateConnectionStatus();

    const kernel = e.kernel ?? cachedKernel;
    const cpuModel = e.cpu_model ?? cachedCpuModel;
    const cpuMhz = e.cpu_mhz ?? cachedCpuMhz;

    if(kernel) updateTextIfChanged('kernelRow', `Linux Kernel: ${kernel}`);
    if(cpuModel) updateTextIfChanged('cpuDetailsRow', `CPU Details: ${cpuModel}${cpuMhz ? `, ${cpuMhz}MHz` : ''}`);

    if(e.cpu !== undefined){
        // Update CPU bar
        const cpuBar = document.getElementById('cpuBar');
        const cpuPct = document.getElementById('cpuPct');
        const color = e.cpu >= 90 ? 'bg-red-500' : e.cpu >= 70 ? 'bg-yellow-500' : 'bg-green-500';
        const widthValue = Math.min(100, e.cpu) + '%';
        updateStyleIfChanged('cpuBar', 'width', widthValue);
        updateIfChanged('cpuBar_class', color, () => {
            cpuBar.className = `block h-full transition-all duration-300 ${color}`;
        });
        updateTextIfChanged('cpuPct', e.cpu.toFixed(1) + '%');

        const loadText = `Load average: ${e.load?.toFixed(2) || '--'} ${e.load5?.toFixed(2) || '--'} ${e.load15?.toFixed(2) || '--'}`;
        updateTextIfChanged('loadVal', loadText);

        // Update CPU history
        cpuHistory.push(e.cpu);
        if(cpuHistory.length > MAX_HISTORY) cpuHistory.shift();
        queueChartUpdate('cpu');
    }
    (e.per_core_cpu || []).forEach((v, i) => updateCoreBar(`core_${i}`, v, document.getElementById('cpuCoresContainer'), i));

    // Update cached total values when present
    if(e.mem_total != null) cachedMemTotal = e.mem_total;
    if(e.swap_total != null) cachedSwapTotal = e.swap_total;
    if(e.disk_total != null) cachedDiskTotal = e.disk_total;
    if(e.filesystems && e.filesystems.length > 0) cachedFilesystems = e.filesystems;
    if(e.net_ip != null) cachedNetIp = e.net_ip;
    if(e.net_gateway != null) cachedNetGateway = e.net_gateway;
    if(e.net_dns != null) cachedNetDns = e.net_dns;

    // Memory display - percentage is always calculated by backend
    if(e.mem !== undefined && e.mem_used !== undefined){
        const memTotal = e.mem_total ?? cachedMemTotal ?? 0;
        updateRamBar(e.mem, e.mem_used, document.getElementById('ramUsed'));
        if(memTotal > 0) {
            const availText = `Available RAM: ${fmt(memTotal - e.mem_used)}`;
            updateTextIfChanged('ramAvail', availText);
        }
        // Update memory history
        memoryHistory.push(e.mem);
        if(memoryHistory.length > MAX_HISTORY) memoryHistory.shift();
        queueChartUpdate('memory');
    }
    if(e.cpu_temp){
        const color = e.cpu_temp >= 80 ? 'text-red-600' : e.cpu_temp >= 60 ? 'text-yellow-600' : 'text-green-600';
        const cpuTempHtml = `CPU Temp <span class="${color}">${Math.round(e.cpu_temp)}°C</span>`;
        updateHtmlIfChanged('cpuTemp', cpuTempHtml);
    } else {
        updateTextIfChanged('cpuTemp', '');
    }
    if(e.mobo_temp){
        const color = e.mobo_temp >= 80 ? 'text-red-600' : e.mobo_temp >= 60 ? 'text-yellow-600' : 'text-green-600';
        const moboTempHtml = `MB Temp <span class="${color}">${Math.round(e.mobo_temp)}°C</span>`;
        updateHtmlIfChanged('moboTemp', moboTempHtml);
    } else if(e.fans && e.fans.length > 0){
        const fan = e.fans[0];
        const fanText = `${fan.label || 'Fan'} ${fan.rpm}RPM`;
        updateTextIfChanged('moboTemp', fanText);
    } else {
        updateTextIfChanged('moboTemp', '');
    }
    // Graphics section - only show if GPU data available
    const hasGpu = e.gpu_freq || e.gpu_temp2 || e.gpu_mem_freq || e.gpu_power;
    const gpuDisplay = hasGpu ? 'flex' : 'none';
    updateStyleIfChanged('graphicsSection', 'display', gpuDisplay);
    updateStyleIfChanged('graphicsRow1', 'display', gpuDisplay);
    updateStyleIfChanged('graphicsRow2', 'display', gpuDisplay);
    if(hasGpu){
        const gpuFreqText = e.gpu_freq ? `GPU Freq ${e.gpu_freq}MHz` : '';
        updateTextIfChanged('gpuFreq', gpuFreqText);
        if(e.gpu_temp2){
            const color = e.gpu_temp2 >= 80 ? 'text-red-600' : e.gpu_temp2 >= 60 ? 'text-yellow-600' : 'text-green-600';
            const gpuTempHtml = `GPU Temp <span class="${color}">${Math.round(e.gpu_temp2)}°C</span>`;
            updateHtmlIfChanged('gpuTemp', gpuTempHtml);
        }
        const memFreqText = e.gpu_mem_freq ? `Mem Freq ${e.gpu_mem_freq}MHz` : '';
        updateTextIfChanged('memFreq', memFreqText);
        const powerText = e.gpu_power ? `Power ${e.gpu_power.toFixed(0)}W` : '';
        updateTextIfChanged('imgQuality', powerText);
    }
    const netInterface = e.net_interface || 'net';

    updateTextIfChanged('netName', `${netInterface}:`);
    updateTextIfChanged('netSpeedDown', `Down: ${fmtRate(e.net_recv || 0)}`);
    updateTextIfChanged('netSpeedUp', `Up: ${fmtRate(e.net_send || 0)}`);

    // Update network history
    netDownHistory.push(e.net_recv || 0);
    if(netDownHistory.length > MAX_HISTORY) netDownHistory.shift();
    queueChartUpdate('netDown');

    netUpHistory.push(e.net_send || 0);
    if(netUpHistory.length > MAX_HISTORY) netUpHistory.shift();
    queueChartUpdate('netUp');

    // Show RX and TX stats with errors/drops
    const rxErrors = e.net_recv_errors || 0;
    const rxDrops = e.net_recv_drops || 0;
    const txErrors = e.net_send_errors || 0;
    const txDrops = e.net_send_drops || 0;

    const rxText = `RX: ${rxErrors} err/s, ${rxDrops} drop/s`;
    const txText = `TX: ${txErrors} err/s, ${txDrops} drop/s`;
    const rxColor = (rxErrors > 0 || rxDrops > 0) ? 'text-red-600' : 'text-gray-500';
    const txColor = (txErrors > 0 || txDrops > 0) ? 'text-red-600' : 'text-gray-500';

    updateTextIfChanged('netRxStats', rxText);
    updateTextIfChanged('netTxStats', txText);
    updateIfChanged('netRxStats_class', rxColor, () => {
        document.getElementById('netRxStats').className = `flex-1 ${rxColor}`;
    });
    updateIfChanged('netTxStats_class', txColor, () => {
        document.getElementById('netTxStats').className = `flex-1 ${txColor}`;
    });

    updateTextIfChanged('netAddress', `Address: ${e.net_ip ?? cachedNetIp ?? '--'}`);
    updateTextIfChanged('netTcp', `TCP Connections: ${e.tcp || '--'}`);
    updateTextIfChanged('netGateway', `Gateway: ${e.net_gateway ?? cachedNetGateway ?? '--'}`);
    updateTextIfChanged('netDns', `DNS: ${e.net_dns ?? cachedNetDns ?? '--'}`);

    // Storage section - use cached filesystems if not in current event or if empty
    const filesystems = (e.filesystems && e.filesystems.length > 0) ? e.filesystems : cachedFilesystems;
    if(filesystems && filesystems.length > 0) {
        filesystems.forEach((fs, i) => {
            const pct = fs.total_bytes > 0 ? Math.round((fs.used_bytes / fs.total_bytes) * 100) : 0;
            updateDiskBar(`disk_${i}`, pct, document.getElementById('diskContainer'), fs.mount_point, fs.used_bytes, fs.total_bytes);
        });
    }

    // Disk IO section
    updateDiskIo(e.per_disk || []);

    // Users section
    const users = e.users || [];
    const usersDisplay = users.length > 0 ? 'flex' : 'none';
    updateStyleIfChanged('usersSection', 'display', usersDisplay);
    const userCountText = users.length > 0 ? `${users.length} logged in` : '';
    updateTextIfChanged('userCount', userCountText);

    // Only update users container if the list actually changed
    const usersKey = JSON.stringify(users);
    if(prevValues['usersContainer_data'] !== usersKey) {
        prevValues['usersContainer_data'] = usersKey;
        const usersContainer = document.getElementById('usersContainer');
        usersContainer.innerHTML = '';
        users.forEach(u => {
            const isRemote = u.remote_host && u.remote_host !== '';
            const div = document.createElement('div');
            div.className = 'text-gray-500 flex justify-between';
            div.innerHTML = `<span>${u.username} <span class="text-gray-400">(${u.terminal})</span></span>${isRemote ? `<span class="text-gray-400">from ${u.remote_host}</span>` : ''}`;
            usersContainer.appendChild(div);
        });
    }
}

function updateProcs(event){
    // Use event processes if available, otherwise fall back to cached
    const processes = (event.processes && event.processes.length > 0) ? event.processes : cachedProcesses;
    const totalProcs = event.total_processes ?? cachedTotalProcesses ?? 0;
    const runningProcs = event.running_processes ?? cachedRunningProcesses ?? 0;

    // Update cache if we got new data
    if(event.processes && event.processes.length > 0) cachedProcesses = event.processes;
    if(event.total_processes != null) cachedTotalProcesses = event.total_processes;
    if(event.running_processes != null) cachedRunningProcesses = event.running_processes;

    const procCountText = `${totalProcs} total ${runningProcs} running`;
    updateTextIfChanged('procCount', procCountText);

    const memTotal = cachedMemTotal || lastStats?.mem_total || 0;
    const topCpu = processes.slice().sort((a,b) => b.cpu_percent - a.cpu_percent).slice(0,5);
    const topMem = processes.slice().sort((a,b) => b.mem_bytes - a.mem_bytes).slice(0,5);

    // Only update tables if process lists actually changed
    const topCpuKey = JSON.stringify(topCpu.map(p => `${p.pid}_${p.cpu_percent}`));
    const topMemKey = JSON.stringify(topMem.map(p => `${p.pid}_${p.mem_bytes}`));

    if(prevValues['topCpuTable_data'] !== topCpuKey) {
        prevValues['topCpuTable_data'] = topCpuKey;
        updateProcTable('topCpuTable', topCpu, memTotal);
    }

    if(prevValues['topMemTable_data'] !== topMemKey) {
        prevValues['topMemTable_data'] = topMemKey;
        updateProcTable('topMemTable', topMem, memTotal);
    }
}

function updateConnectionStatus(){
    const isConnected = ws && ws.readyState === 1;

    // Update header controls visibility
    const headerControls = document.getElementById('headerControls');
    const headerDisconnected = document.getElementById('headerDisconnected');
    if(!isConnected && !playbackMode) {
        headerControls.style.display = 'none';
        headerDisconnected.style.display = 'inline';
    } else {
        headerControls.style.display = 'flex';
        headerDisconnected.style.display = 'none';
    }

    // Update timeDisplay to show "Disconnected" when not connected (only in live mode)
    if(!playbackMode) {
        const timeDisplay = document.getElementById('timeDisplay');
        if(!isConnected) {
            timeDisplay.textContent = 'Disconnected';
            timeDisplay.style.color = '#ef4444'; // red-500
        } else if(timeDisplay.textContent === 'Disconnected') {
            // Restore to "Live" when reconnected
            timeDisplay.textContent = 'Live';
            timeDisplay.style.color = '#6b7280'; // gray-500
        }
    }
}

function connectWebSocket(){
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(protocol + '//' + window.location.host + '/ws');
    ws.onopen = () => {
        updateConnectionStatus();
    };
    ws.onmessage = (ev) => {
        // Fast-path early returns
        if(isPaused || playbackMode) return;

        try {
            const e = JSON.parse(ev.data);
            // Use switch for better performance than if-else chain
            switch(e.type) {
                case 'Metadata':
                    // Populate caches from metadata without rendering
                    if(e.mem_total != null) cachedMemTotal = e.mem_total;
                    if(e.swap_total != null) cachedSwapTotal = e.swap_total;
                    if(e.disk_total != null) cachedDiskTotal = e.disk_total;
                    if(e.net_ip != null) cachedNetIp = e.net_ip;
                    if(e.net_gateway != null) cachedNetGateway = e.net_gateway;
                    if(e.net_dns != null) cachedNetDns = e.net_dns;
                    if(e.kernel != null) cachedKernel = e.kernel;
                    if(e.cpu_model != null) cachedCpuModel = e.cpu_model;
                    if(e.cpu_mhz != null) cachedCpuMhz = e.cpu_mhz;
                    if(e.filesystems && e.filesystems.length > 0) {
                        cachedFilesystems = e.filesystems;
                    }
                    if(e.fans && e.fans.length > 0) cachedFans = e.fans;
                    if(e.processes && e.processes.length > 0) {
                        cachedProcesses = e.processes;
                    }
                    if(e.total_processes != null) cachedTotalProcesses = e.total_processes;
                    if(e.running_processes != null) cachedRunningProcesses = e.running_processes;
                    // Render processes immediately if available (collected every 10s)
                    if(e.processes && e.processes.length > 0) {
                        updateProcs(e);
                    }
                    // Don't render other data - just populate caches for when real data arrives
                    break;
                case 'SystemMetrics':
                    lastStats = e;
                    render();
                    break;
                case 'ProcessSnapshot':
                    updateProcs(e);
                    break;
                default:
                    addEventToLog(e);
            }
        } catch(err) {
            // Silent fail - don't log to avoid console spam
        }
    };
    ws.onerror = () => {
        updateConnectionStatus();
    };
    ws.onclose = () => {
        updateConnectionStatus();
        setTimeout(connectWebSocket, 5000);
    };
}

function addEventToLog(event){
    // Deduplicate: check if this event already exists using O(1) Set lookup
    // Events are considered duplicates if they have the same timestamp, type, and key identifiers
    const eventKey = `${event.timestamp}_${event.type}_${event.pid || event.path || event.message || ''}`;

    if(eventKeys.has(eventKey)) {
        return; // Skip duplicate event
    }

    eventBuffer.push(event);
    eventKeys.add(eventKey);

    if(eventBuffer.length > MAX_BUFFER) {
        const removedEvent = eventBuffer.shift();
        // Remove the key for the shifted event
        const removedKey = `${removedEvent.timestamp}_${removedEvent.type}_${removedEvent.pid || removedEvent.path || removedEvent.message || ''}`;
        eventKeys.delete(removedKey);
    }

    const filter = document.getElementById('filterInput').value.toLowerCase();
    const evType = document.getElementById('eventType').value;
    if(matchesFilter(event, filter, evType)){
        const container = document.getElementById('eventsContainer');
        // Check if user is near bottom before adding (within 50px)
        const wasNearBottom = container.scrollHeight - container.scrollTop - container.clientHeight < 50;
        const entry = createEventEntry(event);
        if(entry){
            // Add new events at the bottom (terminal-style)
            container.appendChild(entry);
            // Remove old events from the top
            if(container.children.length > 200) container.removeChild(container.firstChild);
            // Only auto-scroll if user was already near bottom
            if(wasNearBottom) container.scrollTop = container.scrollHeight;
        }
    }
}

function matchesFilter(e, filter, evType){
    if(evType){
        const map = {process:'ProcessLifecycle', security:'SecurityEvent', anomaly:'Anomaly', filesystem:'FileSystemEvent'};
        if(e.type !== map[evType]) return false;
    }
    return !filter || JSON.stringify(e).toLowerCase().includes(filter);
}

function createEventEntry(e){
    if(!e.type || e.type === 'ProcessSnapshot') return null;
    const div = document.createElement('div');
    div.className = 'text-gray-600 break-all';
    // Format timestamp (now in milliseconds) to HH:MM:SS.mmm
    const time = e.timestamp ? new Date(e.timestamp).toISOString().substring(11,23) : '--:--:--';
    if(e.type === 'ProcessLifecycle'){
        const color = e.kind === 'Started' ? 'text-green-600' : e.kind === 'Exited' ? 'text-gray-400' : 'text-yellow-600';
        // Show full command line inline for forensics
        const cmd = e.cmdline || e.name;
        let details = `(pid ${e.pid}`;
        if(e.ppid) details += `, ppid ${e.ppid}`;
        if(e.user) details += `, user ${e.user}`;
        if(e.working_dir) details += `, cwd ${e.working_dir}`;
        details += ')';
        div.innerHTML = `<span class="text-gray-400">${time}</span> <span class="${color}">[${e.kind}]</span> ${cmd} <span class="text-gray-400">${details}</span>`;
    } else if(e.type === 'SecurityEvent'){
        const color = e.kind.includes('Success') ? 'text-green-600' : 'text-red-600';
        div.innerHTML = `<span class="text-gray-400">${time}</span> <span class="${color}">[${e.kind}]</span> ${e.user} ${e.source_ip ? 'from ' + e.source_ip : ''}`;
    } else if(e.type === 'Anomaly'){
        const color = e.severity === 'Critical' ? 'text-red-600' : 'text-yellow-600';
        div.innerHTML = `<span class="text-gray-400">${time}</span> <span class="${color}">[${e.severity}]</span> ${e.message}`;
    } else if(e.type === 'FileSystemEvent'){
        const color = e.kind === 'Created' ? 'text-blue-600' : e.kind === 'Deleted' ? 'text-red-600' : 'text-yellow-600';
        let sizeInfo = '';
        if(e.size) {
            const fmt = (b) => {
                if(!b) return '0B';
                const k=1024, s=['B','KB','MB','GB','TB'], i=Math.floor(Math.log(b)/Math.log(k));
                return (b/Math.pow(k,i)).toFixed(i>1?1:0)+s[i];
            };
            sizeInfo = ` <span class="text-gray-400">(${fmt(e.size)})</span>`;
        }
        div.innerHTML = `<span class="text-gray-400">${time}</span> <span class="${color}">[${e.kind}]</span> ${e.path}${sizeInfo}`;
    }
    return div;
}

function reloadEvents(){
    const container = document.getElementById('eventsContainer');
    const filter = document.getElementById('filterInput').value.toLowerCase();
    const evType = document.getElementById('eventType').value;

    // Use document fragment for smoother batch update
    const fragment = document.createDocumentFragment();
    eventBuffer.forEach(event => {
        if(matchesFilter(event, filter, evType)){
            const entry = createEventEntry(event);
            if(entry) fragment.appendChild(entry);
        }
    });

    // Replace content in one operation
    container.innerHTML = '';
    container.appendChild(fragment);
    // Scroll to bottom after reload
    container.scrollTop = container.scrollHeight;
}

document.getElementById('filterInput').addEventListener('input', reloadEvents);
document.getElementById('eventType').addEventListener('change', reloadEvents);

// Connect WebSocket (initial state will be sent as first message)
connectWebSocket();

// Redraw timeline on window resize
window.addEventListener('resize', () => {
    drawTimeline();
});

// Only update clock in live mode (when not in playback and we have live data)
setInterval(() => {
    if(!playbackMode && lastStats && lastStats.timestamp) {
        // In live mode, update the display using the live timestamp
        const eventDate = new Date(lastStats.timestamp);
        if(!isNaN(eventDate.getTime())) {
            document.getElementById('datetime').textContent = formatDate(eventDate);
        } else {
            document.getElementById('datetime').textContent = formatDate(new Date());
        }
    }
}, 1000);

</script>
</body>
</html>
"##;
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html)
}

pub async fn api_events(
    reader: web::Data<LogReader>,
    query: web::Query<EventQueryParams>,
) -> HttpResponse {
    let filter = query.filter.as_ref().map(|s| s.to_lowercase());
    let event_type = query.event_type.as_deref();

    let events = match reader.read_all_events() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error reading events: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": format!("Failed to read events: {}", e)}));
        }
    };

    // Convert to JSON-serializable format
    let mut json_events = Vec::new();

    for event in events.iter().rev().take(1000) {
        if let Some(json_event) = event_to_json(event, &filter, event_type) {
            json_events.push(json_event);
        }
    }

    json_events.reverse();

    HttpResponse::Ok().json(json_events)
}

fn event_to_json(
    event: &Event,
    filter: &Option<String>,
    event_type_filter: Option<&str>,
) -> Option<serde_json::Value> {
    use time::format_description::well_known::Rfc3339;

    match event {
        Event::SystemMetrics(m) => {
            if event_type_filter.is_some() && event_type_filter != Some("system") {
                return None;
            }

            // Percentages are now calculated every second in main.rs using cached totals

            Some(serde_json::json!({
                "type": "SystemMetrics",
                "timestamp": m.ts.format(&Rfc3339).ok()?,
                "kernel": m.kernel_version,
                "cpu_model": m.cpu_model,
                "cpu_mhz": m.cpu_mhz,
                "system_uptime_seconds": m.system_uptime_seconds,
                "cpu": m.cpu_usage_percent,
                "per_core_cpu": m.per_core_usage,
                "mem": m.mem_usage_percent,
                "mem_used": m.mem_used_bytes,
                "mem_total": m.mem_total_bytes,
                "load": m.load_avg_1m,
                "load5": m.load_avg_5m,
                "load15": m.load_avg_15m,
                "disk": m.disk_usage_percent.round(),
                "disk_used": m.disk_used_bytes,
                "disk_total": m.disk_total_bytes,
                "per_disk": m.per_disk_metrics.iter().map(|d| serde_json::json!({
                    "device": d.device_name,
                    "read": d.read_bytes_per_sec,
                    "write": d.write_bytes_per_sec,
                    "temp": d.temp_celsius,
                })).collect::<Vec<_>>(),
                "filesystems": m.filesystems.as_ref().map(|fs_list| fs_list.iter().map(|fs| serde_json::json!({
                    "filesystem": fs.filesystem,
                    "mount_point": fs.mount_point,
                    "total_bytes": fs.total_bytes,
                    "used_bytes": fs.used_bytes,
                    "available_bytes": fs.available_bytes,
                })).collect::<Vec<_>>()).unwrap_or_default(),
                "tcp": m.tcp_connections,
                "tcp_wait": m.tcp_time_wait,
                "net_recv": m.net_recv_bytes_per_sec,
                "net_send": m.net_send_bytes_per_sec,
                "net_recv_errors": m.net_recv_errors_per_sec,
                "net_send_errors": m.net_send_errors_per_sec,
                "net_recv_drops": m.net_recv_drops_per_sec,
                "net_send_drops": m.net_send_drops_per_sec,
                "net_interface": m.net_interface,
                "net_ip": m.net_ip_address,
                "net_gateway": m.net_gateway,
                "net_dns": m.net_dns,
                "cpu_temp": m.temps.cpu_temp_celsius,
                "per_core_temps": m.temps.per_core_temps,
                "gpu_temp": m.temps.gpu_temp_celsius,
                "mobo_temp": m.temps.motherboard_temp_celsius,
                "gpu_freq": m.gpu.gpu_freq_mhz,
                "gpu_mem_freq": m.gpu.mem_freq_mhz,
                "gpu_temp2": m.gpu.gpu_temp_celsius,
                "gpu_power": m.gpu.power_watts,
                "fans": m.fans.as_ref().map(|fan_list| fan_list.iter().map(|f| serde_json::json!({
                    "label": f.label,
                    "rpm": f.rpm,
                })).collect::<Vec<_>>()).unwrap_or_default(),
                "users": m.logged_in_users.as_ref().map(|user_list| user_list.iter().map(|u| serde_json::json!({
                    "username": u.username,
                    "terminal": u.terminal,
                    "remote_host": u.remote_host,
                })).collect::<Vec<_>>()).unwrap_or_default(),
            }))
        }
        Event::ProcessLifecycle(p) => {
            if event_type_filter.is_some() && event_type_filter != Some("process") {
                return None;
            }

            let text = format!("{:?} {} {}", p.kind, p.name, p.pid);
            if let Some(f) = filter {
                if !text.to_lowercase().contains(f) {
                    return None;
                }
            }

            Some(serde_json::json!({
                "type": "ProcessLifecycle",
                "timestamp": p.ts.format(&Rfc3339).ok()?,
                "kind": format!("{:?}", p.kind),
                "pid": p.pid,
                "ppid": p.ppid,
                "name": p.name,
                "cmdline": p.cmdline,
                "working_dir": p.working_dir,
                "user": p.user,
                "uid": p.uid,
                "exit_code": p.exit_code,
            }))
        }
        Event::SecurityEvent(s) => {
            if event_type_filter.is_some() && event_type_filter != Some("security") {
                return None;
            }

            let text = format!("{} {} {:?}", s.user, s.message, s.kind);
            if let Some(f) = filter {
                if !text.to_lowercase().contains(f) {
                    return None;
                }
            }

            Some(serde_json::json!({
                "type": "SecurityEvent",
                "timestamp": s.ts.format(&Rfc3339).ok()?,
                "kind": format!("{:?}", s.kind),
                "user": s.user,
                "source_ip": s.source_ip,
                "message": s.message,
            }))
        }
        Event::Anomaly(a) => {
            if event_type_filter.is_some() && event_type_filter != Some("anomaly") {
                return None;
            }

            let text = format!("{:?} {}", a.kind, a.message);
            if let Some(f) = filter {
                if !text.to_lowercase().contains(f) {
                    return None;
                }
            }

            Some(serde_json::json!({
                "type": "Anomaly",
                "timestamp": a.ts.format(&Rfc3339).ok()?,
                "severity": format!("{:?}", a.severity),
                "kind": format!("{:?}", a.kind),
                "message": a.message,
            }))
        }
        Event::ProcessSnapshot(p) => {
            if event_type_filter.is_some() && event_type_filter != Some("process") {
                return None;
            }

            Some(serde_json::json!({
                "type": "ProcessSnapshot",
                "timestamp": p.ts.format(&Rfc3339).ok()?,
                "count": p.processes.len(),
                "total_processes": p.total_processes,
                "running_processes": p.running_processes,
                "processes": p.processes.iter().map(|proc| serde_json::json!({
                    "pid": proc.pid,
                    "name": proc.name,
                    "cmdline": proc.cmdline,
                    "state": proc.state,
                    "user": proc.user,
                    "cpu_percent": proc.cpu_percent,
                    "mem_bytes": proc.mem_bytes,
                    "num_threads": proc.num_threads,
                })).collect::<Vec<serde_json::Value>>(),
            }))
        }
        Event::FileSystemEvent(fse) => {
            if event_type_filter.is_some() && event_type_filter != Some("filesystem") {
                return None;
            }

            let text = format!("{:?} {}", fse.kind, fse.path);
            if let Some(f) = filter {
                if !text.to_lowercase().contains(f) {
                    return None;
                }
            }

            Some(serde_json::json!({
                "type": "FileSystemEvent",
                "timestamp": fse.ts.format(&Rfc3339).ok()?,
                "kind": format!("{:?}", fse.kind),
                "path": fse.path
            }))
        }
    }
}
