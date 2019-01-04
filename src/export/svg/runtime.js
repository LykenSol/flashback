(function() {
    function int(x) {
        return x | 0;
    }

    function svg_element(tag) {
        return document.createElementNS('http://www.w3.org/2000/svg', tag);
    }

    var rt = {};
    rt.mkGlobalScope = function() {
        var o = Object.create(null);
        function def(name, x) {
            Object.defineProperty(o, name, { value: x });
        }
        def('hasOwnProperty', function(o, x) {
            return Object.prototype.hasOwnProperty.call(o, x);
        });
        // HACK(eddyb) trap writes.
        if(Object.freeze)
            return Object.freeze(o);
        return o;
    };
    rt.mkLocalScope = function(_this) {
        var o = Object.create(_this);
        function def(name, x) {
            Object.defineProperty(o, name, { value: x });
        }
        def('this', _this);
        // HACK(eddyb) trap writes.
        if(Object.freeze)
            return Object.freeze(o);
        return o;
    };
    rt.mkMovieClip = function(timeline) {
        var o = Object.create(null);
        function def_get(name, f) {
            Object.defineProperty(o, name, { get: f });
        }
        function def(name, x) {
            Object.defineProperty(o, name, { value: x });
        }
        def('play', function() {
            timeline.paused = false;
        });
        def('stop', function() {
            timeline.paused = true;
        });
        // HACK(eddyb) support for Goto{Frame,Label}.
        def('goto', function(frame) {
            if(typeof frame === 'string')
                frame = timeline.labels[frame];
            timeline.frame = frame;
        });
        def('gotoAndPlay', function(frame) {
            this.goto(frame);
            timeline.paused = false;
        });
        // HACK(eddyb) these are usually only used as the
        // `getBytesLoaded() / getBytesTotal()` ratio.
        def('getBytesLoaded', function() {
            return 1;
        });
        def('getBytesTotal', function() {
            return 1;
        });
        def('getURL', function(url, target) {
            window.open(url, target);
        });
        def_get('_root', rt.mkMovieClip.bind(null, timeline.root));
        if(timeline.parent)
            def_get('_parent', rt.mkMovieClip.bind(null, timeline.parent));
        for(var name in timeline.named) {
            var layer = timeline.layers[timeline.named[name]];
            if(layer && layer.sprite)
                def_get(name, rt.mkMovieClip.bind(null, layer.sprite));
        }
        // HACK(eddyb) trap writes.
        if(Object.freeze)
            return Object.freeze(o);
        return api;
    };

    function Timeline(data, container, id_prefix) {
        if(!(this instanceof Timeline))
            return new Timeline(data);

        id_prefix = id_prefix || '';
        this.frame_count = data.frame_count;
        this.named = Object.create(null);
        this.actions = data.actions;
        this.labels = data.labels;
        this.layers = data.layers.map(function(frames, depth) {
            var container = svg_element('g');
            var use = svg_element('use');
            container.appendChild(use);

            var filter = svg_element('filter');
            filter.setAttribute('id', id_prefix + 'd_' + depth + '_filter');
            filter.setAttribute('x', 0);
            filter.setAttribute('y', 0);
            filter.setAttribute('width', 1);
            filter.setAttribute('height', 1);
            var feColorMatrix = svg_element('feColorMatrix');
            filter.appendChild(feColorMatrix);
            container.appendChild(filter);

            return {
                frames: frames,
                container: container,
                use: use,
                filter: filter,
                feColorMatrix: feColorMatrix,

                ratio: null,

                isHover: function() {
                    // FIXME(eddyb) figure out how much this needs polyfill.
                    return this.container.matches(':hover');
                },

                updateUseHref: function() {
                    if(this.character > 0) {
                        var href = '#c_' + this.character;
                        if(this.button && this.button.state != 'up')
                            href += '_' + this.button.state;
                        if(href != this.useHref)
                            this.use.setAttribute('href', href);
                        this.useHref = href;
                    } else {
                        this.use.removeAttribute('href');
                        this.useHref = null;
                    }
                }
            };
        });
        this.root = this;
        this.container = container;
        this.id_prefix = id_prefix;
        this.attachLayers();
    }
    Timeline.prototype.paused = false;
    Timeline.prototype.frame = 0;
    Timeline.prototype.renderedFrame = -1;
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
    Timeline.prototype.showFrame = function() {
        if(this.paused && this.renderedFrame == this.frame) {
            // Update sprites and buttons even when paused.
            this.layers.forEach(function(layer) {
                if(layer.sprite)
                    layer.sprite.showFrame();
                if(layer.button)
                    layer.button.showFrame();
            });
            return;
        }

        var mkMovieClip = rt.mkMovieClip.bind(null, this);
        var frame = this.frame;
        var renderedFrame = this.renderedFrame;
        var named = this.named;
        var id_prefix = this.id_prefix;

        if(renderedFrame > frame)
            renderedFrame = -1;

        this.layers.forEach(function(layer, depth) {
            var obj, i;
            for(i = frame; i > renderedFrame && !obj && obj !== null; i--)
                obj = layer.frames[i];

            // Fully remove anything not present yet.
            var removeOld = renderedFrame == -1 || obj === null;

            // TODO(eddyb) this might need to take SWF's `is_move` into account.
            // HACK(eddyb) there's the issue of what `ratio` does, see also
            // http://wahlers.com.br/claus/blog/hacking-swf-2-placeobject-and-ratio/.

            // Remove the old character if necessary.
            if(removeOld || (obj && (layer.character != obj.character || layer.ratio !== obj.ratio))) {
                layer.character = -1;
                layer.ratio = null;
                if(layer.sprite) {
                    layer.sprite.detachLayers();
                    layer.sprite.parent = null;
                    layer.sprite.root = null;
                    layer.sprite = null;
                }
                if(layer.button) {
                    layer.button = null;
                }
            }

            // Remove the old name if necessary.
            if(layer.name && (removeOld || (obj && layer.name != obj.name))) {
                named[layer.name] = null;
                layer.name = null;
            }

            if(obj) {
                if(layer.character != obj.character || layer.ratio !== obj.ratio) {
                    layer.character = obj.character;
                    layer.ratio = obj.ratio;

                    var sprite_data = sprites[obj.character];
                    if(sprite_data) {
                        layer.sprite = new Timeline(
                            sprite_data,
                            layer.container,
                            id_prefix + 'd_' + depth + '_',
                        );
                        layer.sprite.parent = this;
                        layer.sprite.root = this.root;
                    }
                    var button_data = buttons[obj.character];
                    if(button_data) {
                        var button = layer.button = {
                            state: 'up',
                            attachListeners: function() {
                                layer.use.addEventListener('mouseover', this.mouse_over_out_up);
                                layer.use.addEventListener('mouseout', this.mouse_over_out_up);
                                layer.use.addEventListener('mouseup', this.mouse_over_out_up);
                                layer.use.addEventListener('mousedown', this.mouse_down);
                            },
                            detachListeners: function() {
                                layer.use.removeEventListener('mouseover', this.mouse_over_out_up);
                                layer.use.removeEventListener('mouseout', this.mouse_over_out_up);
                                layer.use.removeEventListener('mouseup', this.mouse_over_out_up);
                                layer.use.removeEventListener('mousedown', this.mouse_down);
                            },
                            mouse_over_out_up: function(ev) {
                                button.transition(layer.isHover() ? 'over' : 'up');
                            },
                            mouse_down: function() {
                                button.transition('down');
                            },
                            showFrame: function() {
                                if(layer.button !== this)
                                    return;
                                if(!layer.isHover())
                                    this.transition('up');
                            },
                            transition: function(to) {
                                if(layer.button !== this)
                                    return;
                                if(this.state == to)
                                    return;
                                var event;
                                if(this.state == 'up' && to == 'over') {
                                    event = 'hoverIn';
                                } else if(this.state == 'over' && to == 'up') {
                                    event = 'hoverOut';
                                } else if(this.state == 'over' && to == 'down') {
                                    event = 'down';
                                } else if(this.state == 'down' && to == 'over') {
                                    event = 'up';
                                }
                                this.state = to;
                                var handler = event && button_data.mouse[event];
                                if(handler)
                                    handler(rt.mkGlobalScope(), rt.mkLocalScope(mkMovieClip()));
                            },
                        };
                        button.attachListeners();
                    }
                }
                if(obj.matrix) {
                    layer.container.setAttribute('transform', 'matrix(' + obj.matrix.join(' ') + ')');
                } else {
                    layer.container.removeAttribute('transform');
                }
                if(obj.color_transform) {
                    layer.feColorMatrix.setAttribute('values', obj.color_transform.join(' '));
                    layer.container.setAttribute('filter', 'url(#' + layer.filter.id + ')');
                } else {
                    layer.container.removeAttribute('filter');
                }
                if(layer.name != obj.name) {
                    layer.name = obj.name;
                    if(layer.name)
                        named[layer.name] = depth;
                }
            }

            // Update the sprite or button if it exists.
            if(layer.sprite)
                layer.sprite.showFrame();
            if(layer.button)
                layer.button.showFrame();

            // Update the <use> element.
            layer.updateUseHref();
        });

        this.renderedFrame = frame;

        var action = this.actions[frame];
        if(action)
            action(rt.mkGlobalScope(), rt.mkLocalScope(mkMovieClip()));

        // HACK(eddyb) no idea what the interaction here should be.
        if(!this.paused)
            this.frame = (frame + 1) % this.frame_count;
    };

    timeline = new Timeline(timeline, document.getElementById('body'));

    var start;
    var last_frame = 0;
    function update(now) {
        window.requestAnimationFrame(update);

        if(!start) start = now;
        // TODO(eddyb) figure out how to avoid absolute values.
        var frame = int((now - start) * frame_rate / 1000);

        for(; last_frame < frame; last_frame++)
            timeline.showFrame();
    }
    window.requestAnimationFrame(update);
})()
