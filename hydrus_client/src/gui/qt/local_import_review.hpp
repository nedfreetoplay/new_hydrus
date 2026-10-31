#pragma once

// WS-15.1.3c2: local import review dialog moved from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>
#include <QStringList>

class QCheckBox;
class QLabel;
class QPushButton;
class QTableWidget;
class HydrusMainWindow;

// Local import review panel: parse paths, show good/bad/sidecar rows, then
// import the good ones (optionally deleting the originals).
class HydrusLocalImportReviewDialog : public QDialog {
public:
    HydrusLocalImportReviewDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void rebuild();
    void send(const QString &action, const QJsonObject &payload = QJsonObject());
    QStringList selected_paths() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QLabel *summary = nullptr;
    QLabel *warning = nullptr;
    QCheckBox *search = nullptr;
    QCheckBox *delete_after_success = nullptr;
    QPushButton *import_button = nullptr;
    QPushButton *tag_button = nullptr;
};
