(function () {
  var datasets = {
    global: {
      values: [{ label: "2025", value: 447 }, { label: "2026", value: 565 }],
      max: 600,
      ticks: [0, 200, 400, 600],
      suffix: " TWh",
      color: "#4bd3a1"
    },
    ai: {
      values: [{ label: "2025", value: 95 }, { label: "2026", value: 175 }, { label: "2027*", value: 258 }],
      max: 300,
      ticks: [0, 100, 200, 300],
      suffix: " TWh",
      color: "#ff7864"
    }
  };
  var NS = "http://www.w3.org/2000/svg";
  function node(name, attrs, text) {
    var el = document.createElementNS(NS, name);
    Object.keys(attrs || {}).forEach(function (key) { el.setAttribute(key, attrs[key]); });
    if (text !== undefined) el.textContent = text;
    return el;
  }
  function renderChart(id, data) {
    var host = document.getElementById(id);
    var width = 560, height = 270, left = 52, right = 18, top = 22, bottom = 46;
    var plotW = width - left - right, plotH = height - top - bottom;
    var svg = node("svg", { viewBox: "0 0 " + width + " " + height, "aria-hidden": "true", focusable: "false" });
    data.ticks.forEach(function (tick) {
      var y = top + plotH - (tick / data.max) * plotH;
      svg.appendChild(node("line", { x1: left, y1: y, x2: width - right, y2: y, stroke: "currentColor", "stroke-opacity": ".20", "stroke-width": "1" }));
      svg.appendChild(node("text", { x: left - 10, y: y + 4, "text-anchor": "end", fill: "currentColor", "fill-opacity": ".66", "font-size": "11", "font-family": "IBM Plex Mono, monospace" }, String(tick)));
    });
    var slot = plotW / data.values.length;
    var barW = Math.min(96, slot * .48);
    data.values.forEach(function (d, i) {
      var h = (d.value / data.max) * plotH;
      var x = left + slot * i + (slot - barW) / 2;
      var y = top + plotH - h;
      svg.appendChild(node("rect", { x: x, y: y, width: barW, height: h, fill: data.color }));
      svg.appendChild(node("text", { x: x + barW / 2, y: y - 9, "text-anchor": "middle", fill: "currentColor", "font-size": "13", "font-weight": "700", "font-family": "IBM Plex Mono, monospace" }, d.value + data.suffix));
      svg.appendChild(node("text", { x: x + barW / 2, y: height - 17, "text-anchor": "middle", fill: "currentColor", "fill-opacity": ".75", "font-size": "12", "font-family": "IBM Plex Mono, monospace" }, d.label));
    });
    host.appendChild(svg);
  }
  renderChart("chartGlobal", datasets.global);
  renderChart("chartAI", datasets.ai);
  var growth = ((datasets.global.values[1].value / datasets.global.values[0].value - 1) * 100).toFixed(1);
  document.getElementById("energyValue").setAttribute("title", growth + "% increase from 2025");
}());

