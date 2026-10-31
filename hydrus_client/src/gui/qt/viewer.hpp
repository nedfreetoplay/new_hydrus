#pragma once

// WS-15.1.2a: HydrusViewerWindow moved verbatim from bridge.cpp (move-only split).

#include <QJsonObject>
#include <QString>
#include <QWidget>

class QCloseEvent;
class QComboBox;
class QEvent;
class QLabel;
class QPushButton;
class HydrusCanvas;
class HydrusMainWindow;
class HydrusMetadataPanel;

// Media viewer frame: canvas plus a tag sidebar, as in Hydrus' canvas frame.
class HydrusViewerWindow : public QWidget {
public:
    HydrusViewerWindow(HydrusMainWindow *main, const QJsonObject &state);

    const QString &viewer_id() const;

    void update_state(const QJsonObject &state);

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;
    void closeEvent(QCloseEvent *event) override;

private:
    void navigate(const QString &direction);
    void show_hover_frame();

    HydrusMainWindow *main = nullptr;
    QString id;
    QString page_id;
    QString hash;
    HydrusCanvas *canvas = nullptr;
    QWidget *hover_frame = nullptr;
    HydrusMetadataPanel *metadata = nullptr;
    QLabel *status = nullptr;
    QPushButton *animation_button = nullptr;
    QComboBox *background = nullptr;
    QPushButton *pan_button = nullptr;
    QString file_path;
    QString url;
    double zoom_step = 1.25;
    bool always_on_top = false;
    bool frameless = false;
};
