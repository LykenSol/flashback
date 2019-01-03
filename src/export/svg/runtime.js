(function() {
    function int(x) {
        return x | 0;
    }

    var body = document.getElementById('body');
    for(var i = 0; i < timeline.layers.length; i++) {
        var layer = timeline.layers[i];
        if(!layer) continue;
        layer.target = document.createElementNS('http://www.w3.org/2000/svg', 'use');
        body.appendChild(layer.target);
    }

    var start;
    var last_frame;
    var frame;
    function update(now) {
        window.requestAnimationFrame(update);

        if(!start) start = now;
        frame = int((now - start) * frame_rate / 1000) % timeline.frame_count;
        if(frame === last_frame)
            return;

        for(var i = 0; i < timeline.layers.length; i++) {
            var layer = timeline.layers[i];
            if(!layer) continue;
            var obj = layer[frame];
            if(!obj) {
                if(obj === null)
                    layer.target.removeAttribute('href');
                continue;
            }
            layer.target.setAttribute('href', '#c_' + obj.character);
            layer.target.setAttribute('transform', 'matrix(' + obj.matrix.join(' ') + ')');
        }

        last_frame = frame;
    }
    window.requestAnimationFrame(update);
})()
