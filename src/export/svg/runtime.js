(function() {
    function int(x) {
        return x | 0;
    }

    function svg_element(tag) {
        return document.createElementNS('http://www.w3.org/2000/svg', tag);
    }

    function Timeline(data, container) {
        if(!(this instanceof Timeline))
            return new Timeline(data);

        this.frame_count = data.frame_count;
        this.layers = data.layers.map(function(frames, i) {
            var container = svg_element('g');
            var use = svg_element('use');
            container.appendChild(use);
            return { frames: frames, container: container, use: use };
        });
        this.container = container;
        this.attachLayers();
        this.advanceFrames(0);
    }
    Timeline.prototype.frame = 0;
    Timeline.prototype.attachLayers = function() {
        var container = this.container;
        this.layers.forEach(function(layer) {
            container.appendChild(layer.container);
        });
    };
    Timeline.prototype.detachLayers = function() {
        this.layers.forEach(function(layer) {
            layer.container.remove();
        });
    };
    Timeline.prototype.advanceFrames = function(delta) {
        var frame = this.frame = (this.frame + delta) % this.frame_count;
        this.layers.forEach(function(layer) {
            var obj = layer.frames[frame];

            // TODO(eddyb) this might need to take SWF's `is_move` into account.
            // Remove the old character if necessary.
            if(obj === null || (obj && layer.character != obj.character)) {
                layer.character = -1;
                layer.use.removeAttribute('href');
                if(layer.sprite) {
                    layer.sprite.detachLayers();
                    layer.sprite = null;
                }
            } else {
                // Otherwise, update it.
                if(layer.sprite)
                    layer.sprite.advanceFrames(delta);
            }

            if(obj) {
                if(layer.character != obj.character) {
                    layer.character = obj.character;
                    layer.use.setAttribute('href', '#c_' + obj.character);

                    var sprite_data = sprites[obj.character];
                    if(sprite_data)
                        layer.sprite = new Timeline(sprite_data, layer.container);
                }
                layer.container.setAttribute('transform', 'matrix(' + obj.matrix.join(' ') + ')');
            }
        });
    };

    timeline = new Timeline(timeline, document.getElementById('body'));

    var start;
    var last_frame = 0;
    function update(now) {
        window.requestAnimationFrame(update);

        if(!start) start = now;
        // TODO(eddyb) figure out how to avoid absolute values.
        var frame = int((now - start) * frame_rate / 1000);
        var delta = frame - last_frame;
        if(delta)
            timeline.advanceFrames(delta);
        last_frame = frame;
    }
    window.requestAnimationFrame(update);
})()
